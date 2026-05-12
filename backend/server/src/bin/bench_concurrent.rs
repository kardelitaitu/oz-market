use reqwest::header::{HeaderValue, InvalidHeaderValue};
use reqwest::Client;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:3000");
    let total_requests = args
        .get(2)
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| {
            env::var("HTTP_BENCH_OPS")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
        })
        .unwrap_or(1000);
    let concurrency_levels = parse_concurrency_levels(args.get(3).map(|s| s.as_str()));
    let claims_mode = parse_claims_mode(args.get(4).map(|s| s.as_str()));
    let auth = BenchmarkAuth::new(claims_mode)?;

    println!("Real HTTP benchmark against: {}", base_url);
    println!("Requests per level: {}", total_requests);
    println!("Concurrency levels: {:?}", concurrency_levels);
    println!("Claims mode: {}", auth.mode.as_str());
    println!("---");

    let client = Client::builder().pool_max_idle_per_host(1024).build()?;

    run_health_benchmark(&client, base_url).await;

    print_server_config(&client, base_url).await;

    let listing_id = find_listing_id(&client, base_url, &auth).await?;
    println!("Using listing_id: {}", listing_id);

    let search_url = format!("{}/v1/listings/search?limit=20", base_url);

    let cold = measure_single_request(&client, search_url.clone(), &auth, 0).await;
    print_single_result(&cold, "Cold search (first request)");

    println!("\n=== Warm-cache search sweep ===");
    println!("concurrency | success_rate | 429_rate | ops/s | p50_ms | p95_ms | elapsed_ms");
    for concurrency in &concurrency_levels {
        let result = run_benchmark(
            &client,
            search_url.clone(),
            &auth,
            total_requests,
            *concurrency,
        )
        .await;
        print_result_row(*concurrency, &result);
    }

    let max_concurrency = concurrency_levels.iter().copied().max().unwrap_or(50);
    let get_url = format!("{}/v1/listings/{}", base_url, listing_id);

    // Warm listing cache first to avoid connection pool exhaustion
    println!("Warming listing cache...");
    let _ = measure_single_request(&client, get_url.clone(), &auth, 1).await;
    println!("Cache warmed.\n");

    let get_result = run_benchmark(
        &client,
        get_url,
        &auth,
        total_requests.min(1000),
        max_concurrency,
    )
    .await;
    print_named_result("Warm get_listing sample", &get_result);

    println!("\n=== Benchmark Complete ===");
    Ok(())
}

fn parse_concurrency_levels(arg: Option<&str>) -> Vec<usize> {
    let raw = arg
        .map(|s| s.to_string())
        .or_else(|| env::var("HTTP_BENCH_CONCURRENCIES").ok())
        .or_else(|| env::var("HTTP_BENCH_CONCURRENCY").ok())
        .unwrap_or_else(|| "100,200,500".to_string());

    let mut levels = raw
        .split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();

    if levels.is_empty() {
        levels.push(50);
    }

    levels.sort_unstable();
    levels.dedup();
    levels
}

#[derive(Clone, Copy, Debug)]
enum ClaimsMode {
    Public,
    Fixed,
    Rotating,
}

impl ClaimsMode {
    fn as_str(self) -> &'static str {
        match self {
            ClaimsMode::Public => "public",
            ClaimsMode::Fixed => "fixed",
            ClaimsMode::Rotating => "rotating",
        }
    }
}

fn parse_claims_mode(arg: Option<&str>) -> ClaimsMode {
    let raw = arg
        .map(|s| s.to_string())
        .or_else(|| env::var("HTTP_BENCH_CLAIMS_MODE").ok())
        .unwrap_or_else(|| "rotating".to_string());

    match raw.trim().to_ascii_lowercase().as_str() {
        "public" => ClaimsMode::Public,
        "fixed" => ClaimsMode::Fixed,
        "rotating" => ClaimsMode::Rotating,
        _ => ClaimsMode::Rotating,
    }
}

#[derive(Clone)]
struct BenchmarkAuth {
    mode: ClaimsMode,
    fixed_claims: Option<HeaderValue>,
}

impl BenchmarkAuth {
    fn new(mode: ClaimsMode) -> Result<Self, Box<dyn std::error::Error>> {
        let fixed_claims = match mode {
            ClaimsMode::Public => None,
            ClaimsMode::Fixed => Some(benchmark_claims_header()?),
            ClaimsMode::Rotating => None,
        };
        Ok(Self { mode, fixed_claims })
    }

    fn header_for_request(
        &self,
        request_index: u32,
    ) -> Result<Option<HeaderValue>, InvalidHeaderValue> {
        match self.mode {
            ClaimsMode::Public => Ok(None),
            ClaimsMode::Fixed => Ok(self.fixed_claims.clone()),
            ClaimsMode::Rotating => Ok(Some(benchmark_claims_header_for_sub(&format!(
                "bench-searcher-{}",
                request_index
            ))?)),
        }
    }
}

fn benchmark_claims_header() -> Result<HeaderValue, Box<dyn std::error::Error>> {
    if let Ok(raw) = env::var("MARKETPLACE_BENCH_CLAIMS_JSON") {
        return Ok(HeaderValue::from_str(&raw)?);
    }

    Ok(benchmark_claims_header_for_sub("bench-searcher")?)
}

fn benchmark_claims_header_for_sub(sub: &str) -> Result<HeaderValue, InvalidHeaderValue> {
    let claims_json = format!(
        "{{\"sub\":\"{}\",\"roles\":[\"buyer_searcher\"],\"scopes\":[\"listing:read\",\"listing:search\"],\"buyer_agent_id\":\"bench-buyer\"}}",
        sub
    );
    HeaderValue::from_str(&claims_json)
}

async fn run_health_benchmark(client: &Client, base_url: &str) {
    println!("\n=== Health Check ===");
    let auth =
        BenchmarkAuth::new(ClaimsMode::Public).expect("public benchmark auth should not fail");
    let result = run_benchmark(client, format!("{}/health", base_url), &auth, 100, 8).await;
    print_named_result("Health", &result);
}

async fn print_server_config(client: &Client, base_url: &str) {
    println!("\n=== Server Configuration ===");
    let response = match client.get(format!("{}/metrics", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => resp,
        _ => {
            println!("  (metrics endpoint not available)");
            return;
        }
    };

    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => {
            println!("  (failed to read metrics)");
            return;
        }
    };

    // Parse metrics
    let mut worker_threads = "unknown";
    let mut max_worker_threads = "unknown";
    let mut cpu_cores = "unknown";
    let mut listing_max_mb = "unknown";
    let mut search_max_mb = "unknown";
    let mut listing_used_mb = "unknown";
    let mut search_used_mb = "unknown";

    for line in text.lines() {
        if line.starts_with("runtime_worker_threads ") {
            worker_threads = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("runtime_max_worker_threads ") {
            max_worker_threads = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("runtime_cpu_cores ") {
            cpu_cores = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("cache_listing_max_mb ") {
            listing_max_mb = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("cache_search_max_mb ") {
            search_max_mb = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("cache_listing_memory_mb ") {
            listing_used_mb = line.split_whitespace().last().unwrap_or("unknown");
        } else if line.starts_with("cache_search_memory_mb ") {
            search_used_mb = line.split_whitespace().last().unwrap_or("unknown");
        }
    }

    println!("  CPU Cores (Logical): {}", cpu_cores);
    println!("  Tokio Worker Threads (Capped): {}", max_worker_threads);
    println!("  Tokio Worker Threads (Active): {}", worker_threads);
    println!(
        "  Listing Cache: {} MB max, {} MB used",
        listing_max_mb, listing_used_mb
    );
    println!(
        "  Search Cache: {} MB max, {} MB used",
        search_max_mb, search_used_mb
    );
    println!("  Total Cache: used {} MB", listing_used_mb);
}

async fn find_listing_id(
    client: &Client,
    base_url: &str,
    auth: &BenchmarkAuth,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut request = client.get(format!("{}/v1/listings/search?limit=1", base_url));
    let header = auth.header_for_request(0)?;
    if let Some(claims) = header.as_ref() {
        request = request.header("x-marketplace-claims", claims);
    }
    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(format!("search warmup failed with status {}", response.status()).into());
    }

    let json = response.json::<serde_json::Value>().await?;
    let listing_id = json
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .and_then(|first| first.get("listing_id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| "could not find a listing_id in search results".to_string())?;

    Ok(listing_id.to_string())
}

async fn measure_single_request(
    client: &Client,
    url: String,
    auth: &BenchmarkAuth,
    request_index: u32,
) -> BenchmarkResult {
    let start = Instant::now();
    let auth_header = auth
        .header_for_request(request_index)
        .ok()
        .and_then(|value| value);
    let outcome = send_request(client, &url, auth_header.as_ref())
        .await
        .outcome;
    let elapsed = start.elapsed();

    BenchmarkResult {
        total_requests: 1,
        success: if matches!(outcome, RequestOutcome::Success) {
            1
        } else {
            0
        },
        rate_limited: if matches!(outcome, RequestOutcome::RateLimited) {
            1
        } else {
            0
        },
        failed_other: if matches!(outcome, RequestOutcome::OtherFailure) {
            1
        } else {
            0
        },
        elapsed,
        durations_ms: vec![elapsed.as_secs_f64() * 1000.0],
    }
}

async fn run_benchmark(
    client: &Client,
    url: String,
    auth: &BenchmarkAuth,
    total_requests: u32,
    concurrency: usize,
) -> BenchmarkResult {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(total_requests as usize);
    let start = Instant::now();

    for request_index in 0..total_requests {
        let client = client.clone();
        let url = url.clone();
        let auth = auth.clone();
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            let started = Instant::now();
            let auth_header = auth
                .header_for_request(request_index)
                .ok()
                .and_then(|value| value);
            let outcome = send_request(&client, &url, auth_header.as_ref())
                .await
                .outcome;
            (outcome, started.elapsed())
        }));
    }

    let mut success = 0u32;
    let mut rate_limited = 0u32;
    let mut failed_other = 0u32;
    let mut durations = Vec::with_capacity(total_requests as usize);
    for handle in handles {
        if let Ok((outcome, elapsed)) = handle.await {
            match outcome {
                RequestOutcome::Success => success += 1,
                RequestOutcome::RateLimited => rate_limited += 1,
                RequestOutcome::OtherFailure => failed_other += 1,
            }
            durations.push(elapsed.as_secs_f64() * 1000.0);
        } else {
            failed_other += 1;
        }
    }

    let elapsed = start.elapsed();
    BenchmarkResult {
        total_requests,
        success,
        rate_limited,
        failed_other,
        elapsed,
        durations_ms: durations,
    }
}

#[derive(Clone, Copy)]
enum RequestOutcome {
    Success,
    RateLimited,
    OtherFailure,
}

struct RequestResponse {
    outcome: RequestOutcome,
}

async fn send_request(
    client: &Client,
    url: &str,
    auth_header: Option<&HeaderValue>,
) -> RequestResponse {
    let mut request = client.get(url);
    if let Some(header) = auth_header {
        request = request.header("x-marketplace-claims", header);
    }
    match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            let outcome = if status.is_success() {
                RequestOutcome::Success
            } else if status.as_u16() == 429 {
                RequestOutcome::RateLimited
            } else {
                RequestOutcome::OtherFailure
            };
            RequestResponse { outcome }
        }
        Err(_) => RequestResponse {
            outcome: RequestOutcome::OtherFailure,
        },
    }
}

fn print_single_result(result: &BenchmarkResult, title: &str) {
    print_named_result(title, result);
}

fn print_named_result(title: &str, result: &BenchmarkResult) {
    let success_rate = if result.total_requests == 0 {
        0.0
    } else {
        (result.success as f64 / result.total_requests as f64) * 100.0
    };
    let ops_per_sec = if result.elapsed.as_secs_f64() > 0.0 {
        result.success as f64 / result.elapsed.as_secs_f64()
    } else {
        0.0
    };
    println!(
        "{}: {}/{} succeeded ({:.2}%) in {:?}",
        title, result.success, result.total_requests, success_rate, result.elapsed
    );
    println!("  → {:.2} ops/s", ops_per_sec);
    println!(
        "  → 429s: {} | other_failures: {}",
        result.rate_limited, result.failed_other
    );
    println!(
        "  → p50 {:.2} ms | p95 {:.2} ms",
        percentile(&result.durations_ms, 50.0),
        percentile(&result.durations_ms, 95.0)
    );
}

fn print_result_row(concurrency: usize, result: &BenchmarkResult) {
    let success_rate = if result.total_requests == 0 {
        0.0
    } else {
        (result.success as f64 / result.total_requests as f64) * 100.0
    };
    let rate_limited_rate = if result.total_requests == 0 {
        0.0
    } else {
        (result.rate_limited as f64 / result.total_requests as f64) * 100.0
    };
    let ops_per_sec = if result.elapsed.as_secs_f64() > 0.0 {
        result.success as f64 / result.elapsed.as_secs_f64()
    } else {
        0.0
    };
    println!(
        "{:<11} | {:>11.2}% | {:>8.2}% | {:>7.2} | {:>7.2} | {:>7.2} | {:>10.2}",
        concurrency,
        success_rate,
        rate_limited_rate,
        ops_per_sec,
        percentile(&result.durations_ms, 50.0),
        percentile(&result.durations_ms, 95.0),
        result.elapsed.as_secs_f64() * 1000.0,
    );
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = ((percentile / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank]
}

struct BenchmarkResult {
    total_requests: u32,
    success: u32,
    rate_limited: u32,
    failed_other: u32,
    elapsed: Duration,
    durations_ms: Vec<f64>,
}
