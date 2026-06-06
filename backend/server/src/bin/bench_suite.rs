use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use sqlx::postgres::PgPoolOptions;

use oz_market_server::bench::distributed;
use oz_market_server::bench::drivers::{self, DriverConfig};
use oz_market_server::bench::report::{write_report, BenchmarkReport, ThresholdConfig};
use oz_market_server::bench::resource_monitor::ResourceMonitor;
use oz_market_server::bench::scheduler;
use oz_market_server::domain::ledger::CreditLedgerRepository;
use oz_market_server::repositories::ledger::PostgresCreditLedgerRepository;
use oz_market_server::services::ledger_cache::LedgerCache;

#[derive(Parser, Debug)]
#[clap(name = "bench-suite", about = "Backend benchmark suite")]
pub struct Args {
    #[clap(long, default_value = "standalone")]
    pub role: String,

    #[clap(long, default_value = "mock")]
    pub target: String,

    #[clap(long, default_value = "1")]
    pub concurrency: usize,

    #[clap(long)]
    pub rate: Option<u64>,

    #[clap(long, default_value = "10s")]
    pub duration: String,

    /// Database URL (Postgres). Required for --target postgres and --target cache.
    /// Falls back to DATABASE_URL env var if not provided.
    #[clap(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Coordinator listen address (for --role coordinator)
    #[clap(long, default_value = "127.0.0.1:50051")]
    pub addr: String,

    /// Coordinator endpoint to connect to (for --role worker)
    #[clap(long)]
    pub coordinator_addr: Option<String>,

    /// Expected number of workers (for --role coordinator)
    #[clap(long, default_value = "1")]
    pub workers: usize,

    /// Base URL for the running server (for --target http and --target sse).
    /// Defaults to http://127.0.0.1:3000 if not provided.
    #[clap(long, default_value = "http://127.0.0.1:3000")]
    pub base_url: String,

    /// Max connections in the database pool.
    /// Only used when --db-url is provided.
    #[clap(long, default_value = "10")]
    pub db_max_connections: u32,

    /// Write structured JSON report to this file path.
    /// Results include configuration, latency percentiles, and resource metrics.
    #[clap(long)]
    pub report_file: Option<String>,

    /// Run in CI check mode: execute a quick mock-target benchmark and evaluate
    /// thresholds. Exits with code 0 on pass or 1 on failure.
    #[clap(long)]
    pub check: bool,

    /// HTTP driver mode: "health" (default), "search", or "get-listing".
    /// Only used with --target http.
    #[clap(long, default_value = "health")]
    pub http_mode: String,

    /// JSON claims header value for authenticated HTTP requests.
    /// Only used with --target http and --http-mode search or get-listing.
    #[clap(long)]
    pub claims_json: Option<String>,

    /// Enable Postgres search-query mode.
    /// When set, the Postgres driver seeds listings and measures search latency.
    /// Only used with --target postgres.
    #[clap(long)]
    pub pg_search: bool,
}

fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim().to_lowercase();
    if let Some(secs) = s.strip_suffix('s').and_then(|v| v.parse::<u64>().ok()) {
        return Some(Duration::from_secs(secs));
    }
    if let Some(ms) = s.strip_suffix("ms").and_then(|v| v.parse::<u64>().ok()) {
        return Some(Duration::from_millis(ms));
    }
    None
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.check {
        run_check(args).await;
    } else {
        match args.role.as_str() {
            "coordinator" => run_coordinator(args).await,
            "worker" => run_worker(args).await,
            _ => run_standalone(args).await,
        }
    }
}

/// Create a PgPool from a URL string or env var.
async fn maybe_create_pool(
    db_url: &Option<String>,
    db_max_connections: u32,
) -> Option<sqlx::PgPool> {
    let url = db_url.as_ref()?;
    match PgPoolOptions::new()
        .max_connections(db_max_connections)
        .connect(url)
        .await
    {
        Ok(pool) => {
            println!("[bench] Connected to Postgres: {}", url);
            Some(pool)
        }
        Err(e) => {
            eprintln!("[bench] WARNING: Failed to connect to Postgres: {e}");
            None
        }
    }
}

/// Run in standalone mode (all-in-one process).
async fn run_standalone(args: Args) {
    let duration = parse_duration(&args.duration).unwrap_or(Duration::from_secs(10));
    let rate = args.rate.unwrap_or(100);

    // Create optional database pool and cache
    let pool = maybe_create_pool(&args.db_url, args.db_max_connections).await;
    let cache = pool.as_ref().map(|p| {
        let repo: Arc<dyn CreditLedgerRepository> =
            Arc::new(PostgresCreditLedgerRepository::new(p.clone()));
        Arc::new(LedgerCache::new(repo, None))
    });

    // Start resource monitor
    let monitor = ResourceMonitor::new();
    let resource_handle = monitor.start();

    // Create and setup the target driver with optional config
    let config = DriverConfig {
        base_url: Some(args.base_url.clone()),
        claims_json: args.claims_json.clone(),
        http_mode: Some(args.http_mode.clone()),
        pg_search_mode: args.pg_search,
        pool: pool.clone(),
        cache: cache.clone(),
    };
    let driver = drivers::create_driver_with_config(&args.target, &config);
    let _ = driver.setup().await;

    let (histogram, error_count) =
        scheduler::run_rate_loop(driver.clone(), rate, duration, args.concurrency).await;

    let _ = driver.teardown().await;
    monitor.stop();
    let resource_report = resource_handle.join().expect("resource monitor panicked");

    // Print results
    println!("=== Benchmark Results ===");
    println!("Target rate: {} QPS", rate);
    println!("Duration: {:?}", duration);
    println!("Concurrency: {}", args.concurrency);
    println!("Total samples: {}", histogram.len());
    if error_count > 0 {
        println!("Operation errors: {}", error_count);
    }
    println!(
        "P50:  {:>9.3} ms",
        histogram.value_at_percentile(50.0) as f64 / 1000.0
    );
    println!(
        "P95:  {:>9.3} ms",
        histogram.value_at_percentile(95.0) as f64 / 1000.0
    );
    println!(
        "P99:  {:>9.3} ms",
        histogram.value_at_percentile(99.0) as f64 / 1000.0
    );
    println!(
        "P999: {:>9.3} ms",
        histogram.value_at_percentile(99.9) as f64 / 1000.0
    );
    println!();
    println!("=== Resource Report ===");
    println!("Avg CPU: {:.1}%", resource_report.avg_cpu_usage_percent);
    println!(
        "Peak memory: {:.1} MB",
        resource_report.peak_memory_bytes as f64 / 1_048_576.0
    );
    println!(
        "Disk I/O: {:.1} MB read, {:.1} MB written",
        resource_report.disk.total_read_bytes as f64 / 1_048_576.0,
        resource_report.disk.total_written_bytes as f64 / 1_048_576.0
    );
    println!("Samples: {}", resource_report.samples);

    // Write structured JSON report if --report-file was specified
    if let Some(path) = &args.report_file {
        let report = BenchmarkReport::new(
            args.target.clone(),
            rate,
            duration.as_secs_f64(),
            args.concurrency,
            &histogram,
            error_count,
            resource_report,
        );
        write_report(path, &report);
    }
}

/// Run a quick CI check: mock-target benchmark + threshold evaluation.
async fn run_check(args: Args) {
    let duration = Duration::from_secs(3);
    let rate = 100u64;

    println!("==============================");
    println!("Benchmark CI Gate");
    println!("==============================");
    println!(
        "Target: mock | Rate: {} QPS | Duration: {:?}",
        rate, duration
    );

    let monitor = ResourceMonitor::new();
    let resource_handle = monitor.start();

    let driver = drivers::create_driver("mock", None, None, None);
    let _ = driver.setup().await;

    let (histogram, error_count) =
        scheduler::run_rate_loop(Arc::clone(&driver), rate, duration, args.concurrency.max(1))
            .await;

    let _ = driver.teardown().await;
    monitor.stop();
    let resource_report = resource_handle.join().expect("resource monitor panicked");

    let report = BenchmarkReport::new(
        "mock".to_string(),
        rate,
        duration.as_secs_f64(),
        args.concurrency.max(1),
        &histogram,
        error_count,
        resource_report,
    );

    println!();
    println!("=== Results ===");
    println!(
        "Samples: {} | Errors: {}",
        report.total_samples, report.operation_errors
    );
    println!(
        "P50: {:.1} ms | P99: {:.1} ms",
        report.p50_ms, report.p99_ms
    );
    println!(
        "CPU: {:.1}% | Memory: {:.1} MB",
        report.resource.avg_cpu_usage_percent,
        report.resource.peak_memory_bytes as f64 / 1_048_576.0
    );

    // Evaluate against default thresholds
    let thresholds = ThresholdConfig::default();
    let results = report.evaluate(&thresholds);

    println!();
    println!("=== Thresholds ===");
    let all_pass = results.iter().all(|r| r.passed);
    for r in &results {
        let icon = if r.passed { "PASS" } else { "FAIL" };
        println!(
            "  [{icon}] {:<20} actual: {:<14} threshold: {}",
            r.name, r.actual, r.threshold
        );
    }

    if let Some(path) = &args.report_file {
        write_report(path, &report);
    }

    println!();
    if all_pass {
        println!("All thresholds passed.");
        std::process::exit(0);
    } else {
        eprintln!("Some thresholds failed — exiting with code 1.");
        std::process::exit(1);
    }
}

/// Run as the coordinator node in a distributed benchmark.
async fn run_coordinator(args: Args) {
    let addr: SocketAddr = args.addr.parse().expect("invalid --addr, expected IP:port");

    println!("==============================");
    println!("Benchmark Coordinator");
    println!("==============================");
    println!("Listening on: {}", addr);
    println!("Expected workers: {}", args.workers);

    let service = distributed::run_coordinator(addr, args.workers)
        .await
        .expect("coordinator failed");

    // Wait for benchmark to finish (workers disconnect)
    let duration = parse_duration(&args.duration).unwrap_or(Duration::from_secs(30));
    tokio::time::sleep(duration + Duration::from_secs(5)).await;

    // Print merged results
    let hist = service.take_histogram().await;
    println!("\n=== Merged Benchmark Results ===");
    println!("Total samples: {}", hist.len());
    println!(
        "P50:  {:>9.3} ms",
        hist.value_at_percentile(50.0) as f64 / 1000.0
    );
    println!(
        "P95:  {:>9.3} ms",
        hist.value_at_percentile(95.0) as f64 / 1000.0
    );
    println!(
        "P99:  {:>9.3} ms",
        hist.value_at_percentile(99.0) as f64 / 1000.0
    );
    println!(
        "P999: {:>9.3} ms",
        hist.value_at_percentile(99.9) as f64 / 1000.0
    );

    // Write structured JSON report if --report-file was specified
    if let Some(path) = &args.report_file {
        // Coordinator mode has no resource monitor, use a zeroed report
        use oz_market_server::bench::resource_monitor::{DiskMetrics, ResourceReport};
        let resource = ResourceReport {
            avg_cpu_usage_percent: 0.0,
            peak_memory_bytes: 0,
            samples: 0,
            disk: DiskMetrics::default(),
        };
        let report = BenchmarkReport::new(
            args.target.clone(),
            args.rate.unwrap_or(100),
            duration.as_secs_f64(),
            args.concurrency,
            &hist,
            0,
            resource,
        );
        write_report(path, &report);
    }
}

/// Run as a worker node in a distributed benchmark.
async fn run_worker(args: Args) {
    let coordinator_addr = args
        .coordinator_addr
        .clone()
        .unwrap_or_else(|| args.addr.clone());

    let duration = parse_duration(&args.duration).unwrap_or(Duration::from_secs(10));
    let rate = args.rate.unwrap_or(100);

    // Create optional database pool and cache
    let pool = maybe_create_pool(&args.db_url, args.db_max_connections).await;
    let cache = pool.as_ref().map(|p| {
        let repo: Arc<dyn CreditLedgerRepository> =
            Arc::new(PostgresCreditLedgerRepository::new(p.clone()));
        Arc::new(LedgerCache::new(repo, None))
    });

    println!("==============================");
    println!("Benchmark Worker");
    println!("==============================");
    println!("Connecting to coordinator at: {}", coordinator_addr);
    println!("Target: {}", args.target);
    println!("Rate: {} QPS", rate);
    println!("Duration: {:?}", duration);

    // Connect and sync with coordinator
    let mut worker = distributed::WorkerClient::new(format!("http://{}", coordinator_addr));
    worker
        .connect_and_sync()
        .await
        .expect("worker failed to connect and sync");

    // Create driver with real deps if available
    let config = DriverConfig {
        base_url: Some(args.base_url.clone()),
        claims_json: args.claims_json.clone(),
        http_mode: Some(args.http_mode.clone()),
        pg_search_mode: args.pg_search,
        pool: pool.clone(),
        cache: cache.clone(),
    };
    let driver = drivers::create_driver_with_config(&args.target, &config);
    driver.setup().await.expect("driver setup failed");

    worker
        .run_and_stream(driver.clone(), rate, duration, args.concurrency)
        .await
        .expect("worker benchmark failed");

    driver.teardown().await.expect("driver teardown failed");

    println!("[worker] Benchmark complete.");
}
