use futures::StreamExt;
use reqwest::Client;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:3000");
    let concurrency = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);

    println!("SSE Stream Concurrency Benchmark against: {}", base_url);
    println!("Target subscriber count: {}", concurrency);
    println!("---");

    let client = Client::new();
    let sse_url = format!("{}/v1/events/commits", base_url);
    let mock_url = format!("{}/internal/v1/commits/mock", base_url);

    // Coordinate client startup
    let connection_barrier = Arc::new(Barrier::new(concurrency + 1));
    let receive_latencies = Arc::new(Mutex::new(Vec::new()));

    println!(
        "1. Establishing {} concurrent SSE subscriptions...",
        concurrency
    );
    let client_arc = Arc::new(client.clone());

    for _ in 0..concurrency {
        let sse_url = sse_url.clone();
        let client = client_arc.clone();
        let connection_barrier = connection_barrier.clone();
        let receive_latencies = receive_latencies.clone();

        tokio::spawn(async move {
            // Subscribe to SSE
            let res = match client.get(&sse_url).send().await {
                Ok(r) => r,
                Err(_) => {
                    connection_barrier.wait().await;
                    return;
                }
            };

            let mut stream = res.bytes_stream();
            // Signal connection is established — stream is now actively listening
            connection_barrier.wait().await;

            let start_wait = Instant::now();

            while let Some(chunk) = stream.next().await {
                if let Ok(chunk_bytes) = chunk {
                    let text = String::from_utf8_lossy(&chunk_bytes);
                    if text.contains("commit_block") {
                        let latency = start_wait.elapsed();
                        let mut latencies = receive_latencies.lock().await;
                        latencies.push(latency);
                        break;
                    }
                } else {
                    break;
                }
            }
        });
    }

    // Wait for all clients to connect
    println!("Wait for all subscribers to establish connections...");
    connection_barrier.wait().await;
    println!(
        "All {} subscribers connected. Waiting 500ms to stabilize...",
        concurrency
    );
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Trigger mock commits
    println!("\n2. Broadcasting mock transaction commit event...");
    let _trigger_time = Instant::now();
    let payload = serde_json::json!({
        "item": "Benchmark Item Pro",
        "price": 499.0
    });

    let mock_resp = client.post(&mock_url).json(&payload).send().await?;

    if !mock_resp.status().is_success() {
        eprintln!(
            "Failed to trigger mock commit: status {}",
            mock_resp.status()
        );
        return Ok(());
    }

    // Wait up to 3 seconds for propagation
    println!("Waiting for event propagation across subscribers...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let latencies = receive_latencies.lock().await;
    let received_count = latencies.len();

    println!("\n=== Benchmark Results ===");
    println!("Total Subscribers: {}", concurrency);
    println!("Events Received:   {}/{}", received_count, concurrency);

    if received_count > 0 {
        let mut sum = Duration::from_secs(0);
        let mut min = Duration::from_secs(999);
        let mut max = Duration::from_secs(0);

        for &lat in latencies.iter() {
            sum += lat;
            if lat < min {
                min = lat;
            }
            if lat > max {
                max = lat;
            }
        }

        let avg = sum / received_count as u32;
        println!("Min Latency:       {:?}", min);
        println!("Max Latency:       {:?}", max);
        println!("Average Latency:   {:?}", avg);
        println!(
            "Propagation Success: {:.1}%",
            (received_count as f64 / concurrency as f64) * 100.0
        );
    } else {
        println!("❌ No subscribers received the mock commit event.");
    }

    Ok(())
}
