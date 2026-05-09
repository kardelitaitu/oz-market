use reqwest::header::HeaderValue;
use reqwest::Client;
use std::env;
use std::sync::Arc;
use std::time::Instant;
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
        .unwrap_or(10_000);
    let concurrency: usize = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            env::var("HTTP_BENCH_CONCURRENCY")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(50);

    println!("Real HTTP benchmark against: {}", base_url);
    println!("Total requests: {}", total_requests);
    println!("Concurrency: {}", concurrency);
    println!("---");

    let client = Client::new();
    let claims_header = benchmark_claims_header()?;

    run_health_benchmark(&client, base_url, total_requests.min(100)).await;

    let listing_id = find_listing_id(&client, base_url, &claims_header).await?;
    println!("Using listing_id: {}", listing_id);

    let search_url = format!("{}/v1/listings/search?query=Benchmark&listing_type=product&category=laptop&condition=used&limit=20", base_url);
    let search_result = run_benchmark(
        &client,
        "search",
        search_url,
        Some(&claims_header),
        total_requests,
        concurrency,
    )
    .await;
    print_results(&search_result);

    let get_url = format!("{}/v1/listings/{}", base_url, listing_id);
    let get_result = run_benchmark(
        &client,
        "get_listing",
        get_url,
        Some(&claims_header),
        total_requests,
        concurrency,
    )
    .await;
    print_results(&get_result);

    println!("\n=== Benchmark Complete ===");
    Ok(())
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

async fn run_health_benchmark(client: &Client, base_url: &str, health_ops: u32) {
    println!("\n=== Health Check ===");
    let result = run_benchmark(
        client,
        "health",
        format!("{}/health", base_url),
        None,
        health_ops,
        8,
    )
    .await;
    print_results(&result);
}

async fn find_listing_id(
    client: &Client,
    base_url: &str,
    claims_header: &HeaderValue,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client
        .get(format!("{}/v1/listings/search?query=Benchmark&listing_type=product&category=laptop&condition=used&limit=1", base_url))
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

async fn run_benchmark(
    client: &Client,
    label: &str,
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
            let mut request = client.get(&url);
            if let Some(header) = auth_header {
                request = request.header("x-marketplace-claims", header);
            }
            matches!(request.send().await, Ok(resp) if resp.status().is_success())
        }));
    }

    let mut success = 0u32;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            success += 1;
        }
    }

    let elapsed = start.elapsed();
    BenchmarkResult {
        label: label.to_string(),
        total_requests,
        success,
        elapsed,
    }
}

fn print_results(result: &BenchmarkResult) {
    println!(
        "{}: {}/{} succeeded in {:?}",
        result.label, result.success, result.total_requests, result.elapsed
    );
    if result.elapsed.as_secs_f64() > 0.0 {
        let ops_per_sec = result.success as f64 / result.elapsed.as_secs_f64();
        println!("  → {:.2} ops/s", ops_per_sec);
    }
}

struct BenchmarkResult {
    label: String,
    total_requests: u32,
    success: u32,
    elapsed: std::time::Duration,
}
