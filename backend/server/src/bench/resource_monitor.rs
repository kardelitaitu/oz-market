use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::{Disks, System};

// ─── Windows disk I/O via GetProcessIoCounters ──────────────
//
// We define the IO_COUNTERS struct and FFI imports ourselves rather than
// pulling in the windows-sys crate, keeping the dependency tree minimal.

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[cfg(windows)]
extern "system" {
    fn GetCurrentProcess() -> isize;
    fn GetProcessIoCounters(process: isize, io_counters: *mut IoCounters) -> i32;
}

/// Retrieve per-process disk I/O counters on Windows via `GetProcessIoCounters`.
/// Returns `(total_read_bytes, total_written_bytes)` elapsed since the
/// benchmark started (delta between two snapshots, caller manages timing).
/// On non-Windows platforms, returns `(0, 0)`.
#[cfg(windows)]
fn get_process_io_bytes() -> (u64, u64) {
    let mut counters = IoCounters::default();
    // SAFETY: GetProcessIoCounters is a documented Windows API. The handle from
    // GetCurrentProcess() is always valid (pseudo-handle, no ownership needed).
    // `counters` is a stack-allocated, zeroed struct with correct layout.
    let result = unsafe { GetProcessIoCounters(GetCurrentProcess(), &mut counters) };
    if result != 0 {
        (counters.read_transfer_count, counters.write_transfer_count)
    } else {
        (0, 0)
    }
}

/// Non-Windows stub: no disk I/O counters available.
#[cfg(not(windows))]
fn get_process_io_bytes() -> (u64, u64) {
    (0, 0)
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DiskMetrics {
    /// Total disk capacity across all physical disks (bytes). Sampled once at start.
    pub total_space_bytes: u64,
    /// Minimum available disk space observed during the benchmark (bytes).
    /// Shows how much free space was consumed by benchmark writes.
    pub min_available_bytes: u64,
    /// Total bytes read from all I/O operations during the benchmark.
    /// Collected via Windows `GetProcessIoCounters` (includes all I/O, not just disk).
    /// On non-Windows platforms this is always 0.
    pub total_read_bytes: u64,
    /// Total bytes written by all I/O operations during the benchmark.
    /// Collected via Windows `GetProcessIoCounters` (includes all I/O, not just disk).
    /// On non-Windows platforms this is always 0.
    pub total_written_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceReport {
    pub avg_cpu_usage_percent: f32,
    pub peak_memory_bytes: u64,
    pub samples: u64,
    pub disk: DiskMetrics,
}

/// Background hardware resource profiler.
///
/// Samples CPU, memory, disk capacity, and disk I/O at 500ms intervals from
/// a dedicated OS thread. Designed to have minimal overhead (<0.1% CPU) so
/// telemetry doesn't skew benchmark results.
///
/// Cross-platform metrics (via `sysinfo`): CPU usage, memory usage, disk
/// capacity (total/available space).
/// Windows-only metrics (via `GetProcessIoCounters` FFI): per-process disk
/// read/written bytes (includes all I/O, not just disk). On non-Windows
/// platforms these fields report 0.
pub struct ResourceMonitor {
    running: Arc<AtomicBool>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Sum total and available space across all physical disks.
    fn disk_capacity_totals(disks: &Disks) -> (u64, u64) {
        let mut total = 0u64;
        let mut available = u64::MAX;
        for disk in disks.iter() {
            total = total.saturating_add(disk.total_space());
            available = available.min(disk.available_space());
        }
        if available == u64::MAX {
            available = 0;
        }
        (total, available)
    }

    /// Start the background sampler thread.
    ///
    /// Returns a `JoinHandle` that produces a `ResourceReport` when the monitor
    /// is stopped via `stop()`.
    pub fn start(&self) -> thread::JoinHandle<ResourceReport> {
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut sys = System::new_all();
            let mut total_cpu = 0.0_f32;
            let mut peak_memory = 0_u64;
            let mut min_available = u64::MAX;
            let mut samples = 0_u64;

            // Warmup — first CPU sample is always 0
            sys.refresh_cpu_all();
            thread::sleep(Duration::from_millis(100));

            // Capture baseline disk space info
            let mut disks = Disks::new_with_refreshed_list();
            let (total_space, _) = Self::disk_capacity_totals(&disks);

            // Capture baseline disk I/O counters (cumulative since process start)
            let (baseline_read, baseline_written) = get_process_io_bytes();

            while running.load(Ordering::Relaxed) {
                sys.refresh_all();

                total_cpu += sys.global_cpu_usage();

                let memory = sys.used_memory();
                if memory > peak_memory {
                    peak_memory = memory;
                }

                // Refresh disk stats and track minimum available space
                disks.refresh();
                let (_, available) = Self::disk_capacity_totals(&disks);
                if available < min_available {
                    min_available = available;
                }

                samples += 1;
                thread::sleep(Duration::from_millis(500));
            }

            // Capture final disk I/O counters and compute delta
            let (end_read, end_written) = get_process_io_bytes();
            let io_read_delta = end_read.saturating_sub(baseline_read);
            let io_written_delta = end_written.saturating_sub(baseline_written);

            ResourceReport {
                avg_cpu_usage_percent: if samples > 0 {
                    total_cpu / samples as f32
                } else {
                    0.0
                },
                peak_memory_bytes: peak_memory,
                samples,
                disk: DiskMetrics {
                    total_space_bytes: total_space,
                    min_available_bytes: if min_available == u64::MAX {
                        0
                    } else {
                        min_available
                    },
                    total_read_bytes: io_read_delta,
                    total_written_bytes: io_written_delta,
                },
            }
        })
    }

    /// Signal the sampler thread to stop and wait for it to produce a report.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_resource_monitor_starts_and_stops() {
        let monitor = ResourceMonitor::new();
        let handle = monitor.start();

        // Let it sample a few times (must exceed sample interval 500ms + startup 100ms)
        thread::sleep(Duration::from_millis(1500));
        monitor.stop();

        let report = handle.join().expect("sampler thread panicked");
        assert!(report.samples > 0, "should have taken at least one sample");
        // CPU should be a reasonable value
        assert!(
            report.avg_cpu_usage_percent >= 0.0,
            "avg CPU should be non-negative"
        );
        assert!(
            report.peak_memory_bytes > 0,
            "should have measured some memory"
        );
        // Disk space should be > 0 (some drive must exist)
        assert!(
            report.disk.total_space_bytes > 0,
            "should have measured some disk capacity"
        );
        assert!(
            report.disk.min_available_bytes > 0,
            "should have some available disk space"
        );
    }
}
