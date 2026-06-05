# Implementation Notes - Benchmark CLI and Standalone Engine

## CLI Scaffolding and Scheduler

Below is the design for the CLI parser and the Coordinated Omission corrected task scheduler:

```rust
use clap::Parser;
use hdrhistogram::Histogram;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[clap(name = "bench-suite")]
pub struct Args {
    #[clap(long, default_value = "standalone")]
    pub role: String,

    #[clap(long, default_value = "all")]
    pub target: String,

    #[clap(long, default_value = "1")]
    pub concurrency: usize,

    #[clap(long)]
    pub rate: Option<u64>,

    #[clap(long, default_value = "10s")]
    pub duration: String,
}

/// Dispatches operations at fixed time intervals (Coordinated Omission corrected scheduler)
pub async fn run_rate_loop(
    rate_qps: u64,
    duration: Duration,
    histogram: Arc<Mutex<Histogram<u64>>>,
) {
    let interval = Duration::from_micros(1_000_000 / rate_qps);
    let start_time = Instant::now();
    let mut next_tick = Instant::now();

    while start_time.elapsed() < duration {
        // Record schedule tick and capture actual operation start
        let schedule_time = next_tick;
        
        tokio::spawn({
            let hist = Arc::clone(&histogram);
            async move {
                let start = Instant::now();
                // Execute driver operation (mocked or actual)
                let success = execute_mock_op().await;
                
                let latency = start.elapsed();
                // Coordinated Omission: measure from scheduled tick instead of actual start
                let total_delay = Instant::now().duration_since(schedule_time);
                
                if success {
                    let mut lock = hist.lock().unwrap();
                    // Record in microsecond resolution
                    let _ = lock.record(total_delay.as_micros() as u64);
                }
            }
        });

        next_tick += interval;
        let now = Instant::now();
        if next_tick > now {
            tokio::time::sleep(next_tick - now).await;
        }
    }
}

async fn execute_mock_op() -> bool {
    true
}
```
