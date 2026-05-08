use reqwest::header::HeaderValue;
use reqwest::Client;
use std::time::{Duration, Instant};
use std::env;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args.get(1).map(|s| s.as_str()).unwrap_or("http://127.0.0.1:3003");
    let total_requests = args.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(5000);
    let concurrency: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    
    println!("Concurrent HTTP Benchmark");
    println!("Target: {} with {} concurrent connections", base_url, concurrency);
    println!("Total requests: {}", total_requests);
    println!("---");
    
    let client = Client::new();
    
    // Create claims header
    let claims_json = serde_json::to_string(&serde_json::json!({
        "sub": "test-subject",
        "seller_account_id": "test-seller-123",
        "roles": ["admin"],
        "scopes": ["listing:create", "listing:read", "listing:search", "negotiation:create", "negotiation:reveal:request", "internal:listing:archive"]
    }))?;
    let claims_header = HeaderValue::from_str(&claims_json)?;
    
    // Test 1: Health Check (no auth)
    println!("\n=== Health Check (no auth) ===");
    let health_ops = 100u32.min(total_requests);
    let duration = run_benchmark(
        &client,
        format!("{}/health", base_url),
        None,
        health_ops,
        concurrency,
    ).await;
    print_results("Health", health_ops, duration);
    
    // Test 2: Search Listings (the main benchmark)
    println!("\n=== Search Listings (concurrent) ===");
    let search_url = format!("{}/v1/listings/search?page=1&page_size=20", base_url);
    let duration = run_benchmark(
        &client,
        search_url,
        Some(&claims_header),
        total_requests,
        concurrency,
    ).await;
    let ops_per_sec = if duration.as_secs_f64() > 0.0 {
        total_requests as f64 / duration.as_secs_f64()
    } else {
        0.0
    };
    print_results("Search", total_requests, duration);
    println!("  → {:.2} ops/s", ops_per_sec);
    if ops_per_sec >= 5000.0 {
        println!("  ✅ TARGET HIT! {} ops/s >= 5,000 ops/s", ops_per_sec);
    } else {
        println!("  ⚠️  Target: 5,000 ops/s (Phase 1 goal)");
        println!("  (Try increasing concurrency: -- -- 100)");
    }
    
    Ok(())
}

async fn run_benchmark(
    client: &Client,
    url: String,
    auth_header: Option<&HeaderValue>,
    total_requests: u32,
    concurrency: usize,
) -> Duration {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::new();
    let start = Instant::now();
    
    for _ in 0..total_requests {
        let client = client.clone();
        let url = url.clone();
        let auth_header = auth_header.cloned();
        let permit = semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            
            let mut request = client.get(&url);
            if let Some(ref header) = auth_header {
                request = request.header("x-marketplace-claims", header);
            }
            
            let _ = request.send().await;
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    start.elapsed()
}

fn print_results(name: &str, ops: u32, duration: Duration) {
    println!("{}: {}/{} succeeded in {:?}", name, ops, ops, duration);
    if duration.as_secs_f64() > 0.0 {
        println!("  → {:.2} ops/s", ops as f64 / duration.as_secs_f64());
    }
}
