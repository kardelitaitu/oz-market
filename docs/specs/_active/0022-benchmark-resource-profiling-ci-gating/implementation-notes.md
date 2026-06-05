# Implementation Notes - Benchmark Resource Profiling and CI Gating

## System Hardware Profiler Design

Below is the design for the background resource profiler utilizing `sysinfo`:

```rust
use sysinfo::{CpuExt, System, SystemExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;

pub struct ResourceMonitor {
    running: Arc<AtomicBool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceReport {
    pub avg_cpu_usage_percent: f32,
    pub peak_memory_bytes: u64,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self) -> thread::JoinHandle<ResourceReport> {
        let running = Arc::clone(&self.running);
        
        thread::spawn(move || {
            let mut sys = System::new_all();
            let mut total_cpu = 0.0;
            let mut peak_memory = 0;
            let mut samples = 0;

            // Warmup sysinfo cpu stats
            sys.refresh_cpu();
            thread::sleep(Duration::from_millis(100));

            while running.load(Ordering::Relaxed) {
                sys.refresh_all();
                
                total_cpu += sys.global_cpu_info().cpu_usage();
                
                let memory = sys.used_memory();
                if memory > peak_memory {
                    peak_memory = memory;
                }
                
                samples += 1;
                thread::sleep(Duration::from_millis(500));
            }

            ResourceReport {
                avg_cpu_usage_percent: if samples > 0 { total_cpu / samples as f32 } else { 0.0 },
                peak_memory_bytes: peak_memory,
            }
        })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
```
