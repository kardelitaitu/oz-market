use reqwest::header::HeaderValue;
use reqwest::Client;
use std::time::Instant;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args.get(1).map(|s| s.as_str()).unwrap_or("http://127.0.0.1:3000");
    let ops = args.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(500);
    
    println!("HTTP Benchmark against: {}", base_url);
    println!("Target operations: {}", ops);
    println!("---");
    
    let client = Client::new();
    
    // Create test claims JSON for authenticated requests
    let claims_json = serde_json::to_string(&serde_json::json!({
        "sub": "test-subject",
        "seller_account_id": "test-seller-123",
        "roles": ["admin"],
        "scopes": ["listing:create", "listing:read", "listing:search", "negotiation:create", "negotiation:reveal:request", "internal:listing:archive"]
    }))?;
    let claims_header = HeaderValue::from_str(&claims_json)?;
    
    // Test 1: Health Check (no auth needed)
    println!("\n=== Health Check (no auth) ===");
    let health_ops = 100.min(ops);
    let start = Instant::now();
    let mut health_success = 0;
    
    for _ in 0..health_ops {
        match client.get(format!("{}/health", base_url)).send().await {
            Ok(resp) if resp.status().is_success() => health_success += 1,
            _ => {},
        }
    }
    let elapsed = start.elapsed();
    println!("Health: {}/{} succeeded in {:?}", health_success, health_ops, elapsed);
    if elapsed.as_secs_f64() > 0.0 {
        println!("  → {:.2} ops/s", health_success as f64 / elapsed.as_secs_f64());
    }
    
    // Test 2: Search Listings (listing-read profile)
    println!("\n=== Search Listings (listing-read profile) ===");
    let search_ops = ops;
    let start = Instant::now();
    let mut search_success = 0;
    
    for _ in 0..search_ops {
        let url = format!("{}/v1/listings/search?page=1&page_size=20", base_url);
        match client.get(&url)
            .header("x-marketplace-claims", &claims_header)
            .send().await {
            Ok(resp) if resp.status().is_success() => search_success += 1,
            _ => {},
        }
    }
    let elapsed = start.elapsed();
    println!("Search: {}/{} succeeded in {:?}", search_success, search_ops, elapsed);
    if elapsed.as_secs_f64() > 0.0 {
        let ops_per_sec = search_success as f64 / elapsed.as_secs_f64();
        println!("  → {:.2} ops/s", ops_per_sec);
        if ops_per_sec >= 5000.0 {
            println!("  ✅ TARGET HIT! {} ops/s >= 5,000 ops/s", ops_per_sec);
        } else {
            println!("  ⚠️  Target: 5,000 ops/s (Phase 1 goal)");
        }
    }
    
    // Test 3: Try to get a listing by ID (if we can get one from search)
    println!("\n=== Get Listing by ID (cached path) ===");
    let search_url = format!("{}/v1/listings/search?page=1&page_size=1", base_url);
    if let Ok(resp) = client.get(&search_url)
        .header("x-marketplace-claims", &claims_header)
        .send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(listings) = json.get("listings").and_then(|l| l.as_array()) {
                    if let Some(first) = listings.get(0) {
                        if let Some(listing_id) = first.get("listing_id").and_then(|id| id.as_str()) {
                            println!("Found listing_id: {}", listing_id);
                            
                            let get_ops = ops;
                            let start = Instant::now();
                            let mut get_success = 0;
                            
                            for _ in 0..get_ops {
                                let url = format!("{}/v1/listings/{}", base_url, listing_id);
                                if let Ok(resp) = client.get(&url)
                                    .header("x-marketplace-claims", &claims_header)
                                    .send().await {
                                    if resp.status().is_success() {
                                        get_success += 1;
                                    }
                                }
                            }
                            let elapsed = start.elapsed();
                            println!("Get Listing: {}/{} succeeded in {:?}", get_success, get_ops, elapsed);
                            if elapsed.as_secs_f64() > 0.0 {
                                let ops_per_sec = get_success as f64 / elapsed.as_secs_f64();
                                println!("  → {:.2} ops/s", ops_per_sec);
                                if ops_per_sec >= 5000.0 {
                                    println!("  ✅ TARGET HIT! {} ops/s >= 5,000 ops/s", ops_per_sec);
                                } else {
                                    println!("  ⚠️  Target: 5,000 ops/s (Phase 1 goal)");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("\n=== Benchmark Complete ===");
    Ok(())
}
