use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;

use super::driver::BenchmarkDriver;

/// Dispatches operations at fixed time intervals (Coordinated Omission corrected).
///
/// Operations are scheduled at `rate_qps` intervals regardless of when the previous
/// operation completed. Each operation calls `driver.run_operation()`. Latency is
/// measured from the scheduled tick time, not the actual operation start time, to
/// avoid coordinated omission bias.
///
/// For concurrent operation, pass `concurrency > 1` — a semaphore controls how many
/// in-flight operations are allowed at once. Each operation is spawned as a separate
/// tokio task with the `driver` Arc cloned for the task.
///
/// Returns a tuple of (histogram, error_count).
pub async fn run_rate_loop(
    driver: Arc<dyn BenchmarkDriver>,
    rate_qps: u64,
    duration: Duration,
    concurrency: usize,
) -> (Histogram<u64>, u64) {
    if rate_qps == 0 {
        return empty_result();
    }
    if concurrency == 0 {
        return empty_result();
    }

    let interval = Duration::from_micros(1_000_000 / rate_qps);
    let start_time = Instant::now();
    let mut next_tick = Instant::now();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let result_histogram: Arc<Mutex<Histogram<u64>>> = Arc::new(Mutex::new(
        Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).expect("valid histogram bounds"),
    ));
    let error_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

    while start_time.elapsed() < duration {
        let schedule_time = next_tick;
        let hist = Arc::clone(&result_histogram);
        let driver = Arc::clone(&driver);
        let sem = Arc::clone(&semaphore);
        let errs = Arc::clone(&error_count);

        tokio::spawn(async move {
            // Acquire concurrency permit — may block if at capacity.
            // Coordinated Omission: measure from the scheduled tick regardless
            // of when we actually acquire the permit and start.
            let _permit = sem.acquire().await;

            let result = driver.run_operation().await;
            let total_delay = Instant::now().duration_since(schedule_time);

            if let Err(e) = result {
                errs.fetch_add(1, Ordering::Relaxed);
                if errs.load(Ordering::Relaxed) <= 5 {
                    eprintln!("[scheduler] Operation error: {e}");
                }
            }

            // Always record latency (even on error) — coordinated omission
            let mut lock = hist.lock().unwrap();
            let _ = lock.record(total_delay.as_micros() as u64);
        });

        next_tick += interval;
        let now = Instant::now();
        if next_tick > now {
            tokio::time::sleep(next_tick - now).await;
        }
    }

    // Extract the histogram from the Arc (may still have task references)
    let hist = match Arc::try_unwrap(result_histogram) {
        Ok(mutex) => mutex.into_inner().unwrap_or_else(|e| e.into_inner()),
        Err(arc) => {
            // Spawned tasks still running — clone the current data
            let guard = arc.lock().unwrap();
            guard.clone()
        }
    };

    // Extract the error count
    let errs = match Arc::try_unwrap(error_count) {
        Ok(atomic) => atomic.load(Ordering::SeqCst),
        Err(arc) => arc.load(Ordering::SeqCst),
    };

    (hist, errs)
}

fn empty_result() -> (Histogram<u64>, u64) {
    (
        Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).expect("valid histogram bounds"),
        0,
    )
}
