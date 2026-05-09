use reqwest::header::HeaderValue;
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

    println!("Real HTTP benchmark against: {}", base_url);
    println!("Requests per level: {}", total_requests);
    println!("Concurrency levels: {:?}", concurrency_levels);
    println!("---");

    let client = Client::builder().pool_max_idle_per_host(1024).build()?;
    let claims_header = benchmark_claims_header()?;

    run_health_benchmark(&client, base_url).await;

    let listing_id = find_listing_id(&client, base_url, &claims_header).await?;
    println!("Using listing_id: {}", listing_id);

    let search_url = format!(
        "{}/v1/listings/search?query=Benchmark&listing_type=product&category=laptop&condition=used&limit=20",
        base_url
    );

    let cold = measure_single_request(&client, search_url.clone(), Some(&claims_header)).await;
    print_single_result(&cold, "Cold search (first request)");

    println!("\n=== Warm-cache search sweep ===");
    println!("concurrency | success_rate | ops/s | p50_ms | p95_ms | elapsed_ms");
    for concurrency in &concurrency_levels {
        let result = run_benchmark(
            &client,
            search_url.clone(),
            Some(&claims_header),
            total_requests,
            *concurrency,
        )
        .await;
        print_result_row(*concurrency, &result);
    }

    let max_concurrency = concurrency_levels.iter().copied().max().unwrap_or(50);
    let get_url = format!("{}/v1/listings/{}", base_url, listing_id);
    let get_result = run_benchmark(
        &client,
        get_url,
        Some(&claims_header),
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
        .unwrap_or_else(|| "1,10,50,100,250,500,1000".to_string());

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

fn benchmark_claims_header() -> Result<HeaderValue, Box<dyn std::error::Error>> {
    if let Ok(raw) = env::var("MARKETPLACE_BENCH_CLAIMS_JSON") {
        return Ok(HeaderValue::from_str(&raw)?);
    }

    let claims_json = serde_json::to_string(&serde_json::json!({
        "sub": "bench-searcher",
        "roles": ["buyer_searcher"],
        "scopes": ["listing:read", "listing:search"],
        "buyer_agent_id": "bench-buyer"
    }))?;
    Ok(HeaderValue::from_str(&claims_json)?)
}

async fn run_health_benchmark(client: &Client, base_url: &str) {
    println!("\n=== Health Check ===");
    let result = run_benchmark(client, format!("{}/health", base_url), None, 100, 8).await;
    print_named_result("Health", &result);
}

async fn find_listing_id(
    client: &Client,
    base_url: &str,
    claims_header: &HeaderValue,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client
        .get(format!(
            "{}/v1/listings/search?query=Benchmark&listing_type=product&category=laptop&condition=used&limit=1",
            base_url
        ))
        .header("x-marketplace-claims", claims_header)
        .send()
        .await?;

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
    auth_header: Option<&HeaderValue>,
) -> BenchmarkResult {
    let start = Instant::now();
    let success = send_request(client, &url, auth_header).await;
    let elapsed = start.elapsed();

    BenchmarkResult {
        total_requests: 1,
        success: if success { 1 } else { 0 },
        elapsed,
        durations_ms: vec![elapsed.as_secs_f64() * 1000.0],
    }
}

async fn run_benchmark(
    client: &Client,
    url: String,
    auth_header: Option<&HeaderValue>,
    total_requests: u32,
    concurrency: usize,
) -> BenchmarkResult {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(total_requests as usize);
    let start = Instant::now();

    for _ in 0..total_requests {
        let client = client.clone();
        let url = url.clone();
        let auth_header = auth_header.cloned();
        let permit = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            let started = Instant::now();
            let success = send_request(&client, &url, auth_header.as_ref()).await;
            (success, started.elapsed())
        }));
    }

    let mut success = 0u32;
    let mut durations = Vec::with_capacity(total_requests as usize);
    for handle in handles {
        if let Ok((ok, elapsed)) = handle.await {
            if ok {
                success += 1;
            }
            durations.push(elapsed.as_secs_f64() * 1000.0);
        }
    }

    let elapsed = start.elapsed();
    BenchmarkResult {
        total_requests,
        success,
        elapsed,
        durations_ms: durations,
    }
}

async fn send_request(client: &Client, url: &str, auth_header: Option<&HeaderValue>) -> bool {
    let mut request = client.get(url);
    if let Some(header) = auth_header {
        request = request.header("x-marketplace-claims", header);
    }
    matches!(request.send().await, Ok(resp) if resp.status().is_success())
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
    let ops_per_sec = if result.elapsed.as_secs_f64() > 0.0 {
        result.success as f64 / result.elapsed.as_secs_f64()
    } else {
        0.0
    };
    println!(
        "{:<11} | {:>11.2}% | {:>7.2} | {:>7.2} | {:>7.2} | {:>10.2}",
        concurrency,
        success_rate,
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
    elapsed: Duration,
    durations_ms: Vec<f64>,
}
