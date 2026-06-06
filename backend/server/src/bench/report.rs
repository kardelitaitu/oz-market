use std::time::{SystemTime, UNIX_EPOCH};

use hdrhistogram::Histogram;
use serde::Serialize;

use super::resource_monitor::ResourceReport;

/// Structured benchmark report serializable to JSON.
///
/// Contains all benchmark configuration, latency percentiles, and
/// resource profiling data in a single flat struct for CI gating
/// and historical comparison.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    // Metadata
    pub timestamp_epoch_ms: u64,
    pub target: String,

    // Configuration
    pub rate_qps: u64,
    pub duration_secs: f64,
    pub concurrency: usize,

    // Results
    pub total_samples: u64,
    pub operation_errors: u64,

    // Latency percentiles (microseconds)
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,

    // Latency percentiles (milliseconds, convenience)
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub p999_ms: f64,

    // Resource profiling
    pub resource: ResourceReport,
}

impl BenchmarkReport {
    pub fn new(
        target: String,
        rate_qps: u64,
        duration_secs: f64,
        concurrency: usize,
        histogram: &Histogram<u64>,
        error_count: u64,
        resource: ResourceReport,
    ) -> Self {
        let extract = |percentile: f64| -> u64 { histogram.value_at_percentile(percentile) };

        let p50_us = extract(50.0);
        let p95_us = extract(95.0);
        let p99_us = extract(99.0);
        let p999_us = extract(99.9);

        let timestamp_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            timestamp_epoch_ms,
            target,
            rate_qps,
            duration_secs,
            concurrency,
            total_samples: histogram.len(),
            operation_errors: error_count,
            p50_us,
            p95_us,
            p99_us,
            p999_us,
            p50_ms: p50_us as f64 / 1000.0,
            p95_ms: p95_us as f64 / 1000.0,
            p99_ms: p99_us as f64 / 1000.0,
            p999_ms: p999_us as f64 / 1000.0,
            resource,
        }
    }

    /// Serialize this report to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, std::io::Error> {
        serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(format!("JSON serialization: {e}")))
    }
}

/// Configuration for pass/fail threshold evaluation.
///
/// Each field can be set to 0 to skip that check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThresholdConfig {
    /// Maximum P99 latency in microseconds (0 = skip)
    pub max_p99_us: u64,
    /// Maximum operation error rate as fraction 0.0–1.0 (0 = skip)
    pub max_error_rate: f64,
    /// Maximum average CPU usage percent (0 = skip)
    pub max_cpu_percent: f32,
    /// Maximum peak memory in bytes (0 = skip)
    pub max_memory_bytes: u64,
    /// Minimum number of samples (0 = skip)
    pub min_samples: u64,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            max_p99_us: 5_000_000, // 5 s (CI/Windows overhead)
            max_error_rate: 0.05,  // 5%
            max_cpu_percent: 90.0,
            max_memory_bytes: 256_000_000_000, // 256 GB (system-wide)
            min_samples: 10,
        }
    }
}

/// Result of a single threshold check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThresholdResult {
    pub name: String,
    pub passed: bool,
    pub actual: String,
    pub threshold: String,
}

impl BenchmarkReport {
    /// Evaluate benchmark results against threshold configuration.
    ///
    /// Returns a list of check results. The benchmark as a whole passes if
    /// every check `passed` is true.
    pub fn evaluate(&self, thresholds: &ThresholdConfig) -> Vec<ThresholdResult> {
        let mut results = Vec::new();

        if thresholds.max_p99_us > 0 {
            let passed = self.p99_us <= thresholds.max_p99_us;
            results.push(ThresholdResult {
                name: "P99 Latency".into(),
                passed,
                actual: format!("{} μs", self.p99_us),
                threshold: format!("≤ {} μs", thresholds.max_p99_us),
            });
        }

        if thresholds.max_error_rate > 0.0 {
            let rate = if self.total_samples > 0 {
                self.operation_errors as f64 / self.total_samples as f64
            } else {
                1.0
            };
            let passed = rate <= thresholds.max_error_rate;
            results.push(ThresholdResult {
                name: "Error Rate".into(),
                passed,
                actual: format!("{:.2}%", rate * 100.0),
                threshold: format!("≤ {:.1}%", thresholds.max_error_rate * 100.0),
            });
        }

        if thresholds.max_cpu_percent > 0.0 {
            let passed = self.resource.avg_cpu_usage_percent <= thresholds.max_cpu_percent;
            results.push(ThresholdResult {
                name: "Avg CPU".into(),
                passed,
                actual: format!("{:.1}%", self.resource.avg_cpu_usage_percent),
                threshold: format!("≤ {:.1}%", thresholds.max_cpu_percent),
            });
        }

        if thresholds.max_memory_bytes > 0 {
            let passed = self.resource.peak_memory_bytes <= thresholds.max_memory_bytes;
            results.push(ThresholdResult {
                name: "Peak Memory".into(),
                passed,
                actual: format!(
                    "{:.1} MB",
                    self.resource.peak_memory_bytes as f64 / 1_048_576.0
                ),
                threshold: format!(
                    "≤ {:.1} MB",
                    thresholds.max_memory_bytes as f64 / 1_048_576.0
                ),
            });
        }

        if thresholds.min_samples > 0 {
            let passed = self.total_samples >= thresholds.min_samples;
            results.push(ThresholdResult {
                name: "Min Samples".into(),
                passed,
                actual: format!("{}", self.total_samples),
                threshold: format!("≥ {}", thresholds.min_samples),
            });
        }

        results
    }
}

/// Write a benchmark report to a JSON file.
///
/// Creates parent directories if they don't exist. Prints a confirmation
/// line on success or a warning on failure (non-fatal — the benchmark
/// results have already been displayed).
pub fn write_report(path: &str, report: &BenchmarkReport) {
    let json = match report.to_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[bench] WARNING: Failed to serialize report: {e}");
            return;
        }
    };

    // Create parent directories if needed
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    match std::fs::write(path, &json) {
        Ok(_) => println!("[bench] Report written to: {path}"),
        Err(e) => eprintln!("[bench] WARNING: Failed to write report to {path}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::resource_monitor::DiskMetrics;
    use super::*;
    use hdrhistogram::Histogram;

    fn make_histogram() -> Histogram<u64> {
        let mut h = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap();
        for v in &[1000, 2000, 3000, 5000, 10_000] {
            h.record(*v).unwrap();
        }
        h
    }

    #[test]
    fn test_benchmark_report_construction() {
        let res = ResourceReport {
            avg_cpu_usage_percent: 25.5,
            peak_memory_bytes: 1_048_576_000,
            samples: 10,
            disk: DiskMetrics::default(),
        };
        let hist = make_histogram();
        let report = BenchmarkReport::new("mock".to_string(), 500, 10.0, 4, &hist, 0, res);

        assert_eq!(report.target, "mock");
        assert_eq!(report.rate_qps, 500);
        assert_eq!(report.total_samples, 5);
        assert_eq!(report.operation_errors, 0);
        assert!(report.p50_ms > 0.0);
        assert!(report.p99_ms >= report.p50_ms);
    }

    #[test]
    fn test_report_serializes_to_json() {
        let res = ResourceReport {
            avg_cpu_usage_percent: 30.0,
            peak_memory_bytes: 2_000_000_000,
            samples: 20,
            disk: DiskMetrics::default(),
        };
        let hist = make_histogram();
        let report = BenchmarkReport::new("postgres".to_string(), 1000, 5.0, 8, &hist, 0, res);

        let json = report.to_json().expect("should serialize");
        assert!(json.contains("\"target\": \"postgres\""));
        assert!(json.contains("\"rate_qps\": 1000"));
        assert!(json.contains("\"avg_cpu_usage_percent\": 30.0"));
    }

    #[test]
    fn test_write_report_creates_file() {
        let res = ResourceReport {
            avg_cpu_usage_percent: 10.0,
            peak_memory_bytes: 512_000_000,
            samples: 5,
            disk: DiskMetrics::default(),
        };
        let hist = make_histogram();
        let report = BenchmarkReport::new("wal".to_string(), 200, 3.0, 2, &hist, 1, res);

        let tmp = std::env::temp_dir().join("oz_bench_test_report.json");
        let path_str = tmp.to_str().unwrap().to_string();
        write_report(&path_str, &report);

        assert!(tmp.exists(), "report file should exist");
        let contents = std::fs::read_to_string(&path_str).unwrap();
        assert!(contents.contains("\"target\": \"wal\""));

        // Clean up
        let _ = std::fs::remove_file(&path_str);
    }
}
