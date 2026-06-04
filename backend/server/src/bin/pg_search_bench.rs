//! Postgres search latency benchmark.
//!
//! For each dataset size: truncates listings, seeds N listings, then measures
//! `PostgresListingRepository::search_listings()` latency. Each size gets
//! a clean database state for accurate per-size measurements.
//!
//! Usage:
//!   DATABASE_URL="postgres://..." cargo run --bin pg_search_bench --release
//!   DATABASE_URL="postgres://..." cargo run --bin pg_search_bench --release -- --sizes 100,500,1000 --warmup 20 --samples 100

use marketplace_api_contract::{
    Category, Condition, CreateListingRequest, ListingLocation, ListingPayload, ListingType, Price,
    SearchRequest, SearchSort,
};
use marketplace_server::repositories::listings::PostgresListingRepository;
use marketplace_server::repositories::ListingRepository;
use sqlx::postgres::PgPoolOptions;
use std::time::Instant;

#[derive(Debug)]
struct BenchResult {
    name: String,
    dataset_size: usize,
    samples: usize,
    mean_us: f64,
    min_us: f64,
    max_us: f64,
    stddev_us: f64,
    matching_listings: usize,
}

fn make_bench_listing_request(seed: u64) -> CreateListingRequest {
    let categories = [
        Category::Laptop,
        Category::Phone,
        Category::Tablet,
        Category::Monitor,
        Category::Audio,
    ];
    let titles = [
        "MacBook Pro 16 inch M3 Max",
        "iPhone 15 Pro Max 256GB",
        "Vintage Leather Jacket Size M",
        "Gaming Desktop PC RTX 4090",
        "Acoustic Guitar Martin D-28",
    ];
    let cities = ["New York", "London", "Tokyo", "Berlin", "San Francisco"];
    let idx = (seed % 5) as usize;

    CreateListingRequest {
        idempotency_key: format!("pg-bench-idem-{seed}"),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: format!("pg_seller_{}", seed % 20),
            listing_type: ListingType::Product,
            category: Some(categories[idx]),
            title: titles[idx].to_string(),
            condition: Some(Condition::New),
            price: Price {
                amount: (seed as f64 * 123.45).rem_euclid(10000.0) + 1.0,
                currency: "USD".to_string(),
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: cities[idx].to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec![],
            description: format!("A high-quality {} in excellent condition", titles[idx]),
            attributes: None,
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: None,
            hourly_rate: None,
            project_rate: None,
            qualifications: None,
            service_radius_km: None,
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        },
    }
}

/// Truncate all bench listings and seed exactly `count` new ones.
async fn reseed_listings(
    repo: &PostgresListingRepository,
    count: usize,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("DELETE FROM listings WHERE owner_id LIKE 'pg_seller_%'")
        .execute(pool)
        .await?;

    eprint!("Seeding {count} listings... ");
    let batch_size = 50;
    for batch_start in (0..count).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(count);
        let requests: Vec<CreateListingRequest> = (batch_start..batch_end)
            .map(|i| make_bench_listing_request(i as u64))
            .collect();
        let mut handles = Vec::with_capacity(requests.len());
        for req in &requests {
            handles.push(repo.insert_listing(req));
        }
        for result in futures::future::join_all(handles).await {
            result?;
        }
    }

    // Verify count
    let row_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM listings WHERE owner_id LIKE 'pg_seller_%'")
            .fetch_one(pool)
            .await?;
    eprintln!("done ({} rows)", row_count.0);

    Ok(())
}

/// Count how many listings match the benchmark search query.
async fn count_matching(pool: &sqlx::PgPool) -> Result<usize, Box<dyn std::error::Error>> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM listings WHERE owner_id LIKE 'pg_seller_%' AND category = 'laptop'",
    )
    .fetch_one(pool)
    .await?;
    Ok(count.0 as usize)
}

async fn run_benchmark(
    repo: &PostgresListingRepository,
    dataset_size: usize,
    samples: usize,
    warmup: usize,
    pool: &sqlx::PgPool,
) -> Result<BenchResult, Box<dyn std::error::Error>> {
    // Clean seed for this size
    reseed_listings(repo, dataset_size, pool).await?;
    let matching = count_matching(pool).await?;

    let search_request = SearchRequest {
        query: Some("MacBook".to_string()),
        category: Some(Category::Laptop),
        sort_by: SearchSort::Relevance,
        limit: Some(50),
        ..Default::default()
    };

    // Warmup
    for _ in 0..warmup {
        repo.search_listings(&search_request).await?;
    }

    // Benchmark
    let mut times_us = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let _ = repo.search_listings(&search_request).await?;
        let elapsed_us = start.elapsed().as_micros() as f64;
        times_us.push(elapsed_us);
    }

    let mean_us = times_us.iter().sum::<f64>() / times_us.len() as f64;
    let min_us = times_us.iter().cloned().fold(f64::MAX, f64::min);
    let max_us = times_us.iter().cloned().fold(f64::MIN, f64::max);
    let variance =
        times_us.iter().map(|t| (t - mean_us).powi(2)).sum::<f64>() / times_us.len() as f64;
    let stddev_us = variance.sqrt();

    eprintln!(
        "pg_search_{dataset_size}: mean={mean_us:.0}µs ±{stddev_us:.0}µs (min={min_us:.0}, max={max_us:.0}, samples={samples}, matching_listings={matching})"
    );

    Ok(BenchResult {
        name: format!("pg_search_{dataset_size}"),
        dataset_size,
        samples,
        mean_us,
        min_us,
        max_us,
        stddev_us,
        matching_listings: matching,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let sizes: Vec<usize> = if let Some(pos) = args.iter().position(|a| a == "--sizes") {
        args[pos + 1]
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    } else {
        vec![100, 500, 1000]
    };

    let samples: usize = if let Some(pos) = args.iter().position(|a| a == "--samples") {
        args[pos + 1].parse().unwrap_or(100)
    } else {
        100
    };

    let warmup: usize = if let Some(pos) = args.iter().position(|a| a == "--warmup") {
        args[pos + 1].parse().unwrap_or(20)
    } else {
        20
    };

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string()
    });

    eprintln!("PostgreSQL search benchmark");
    eprintln!("  Database: {database_url}");
    eprintln!("  Dataset sizes: {sizes:?}");
    eprintln!("  Samples per size: {samples}");
    eprintln!("  Warmup iterations: {warmup}");
    eprintln!();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let repo = PostgresListingRepository::new(pool.clone());

    eprintln!("--- Benchmarking Postgres search_listings() ---");

    let mut results = Vec::new();
    for &size in &sizes {
        let result = run_benchmark(&repo, size, samples, warmup, &pool).await?;
        results.push(result);
    }

    // Print CSV summary
    println!("\n=== Results ===");
    println!("benchmark,dataset_size,samples,mean_us,min_us,max_us,stddev_us,matching");
    for r in &results {
        println!(
            "{},{},{},{:.0},{:.0},{:.0},{:.0},{}",
            r.name,
            r.dataset_size,
            r.samples,
            r.mean_us,
            r.min_us,
            r.max_us,
            r.stddev_us,
            r.matching_listings
        );
    }

    // Print human-readable summary
    println!("\n=== Postgres Search Latency (end-to-end: SQL query + row deserialization + Rust sort + pagination) ===");
    println!();
    println!(
        "  {:<25} {:>12} {:>10} {:>10} {:>10} {:>10}",
        "Benchmark", "Mean", "Min", "Max", "±Stddev", "Match"
    );
    println!(
        "  {:-<25} {:-<12} {:-<10} {:-<10} {:-<10} {:-<10}",
        "", "", "", "", "", ""
    );
    for r in &results {
        println!(
            "  {:<25} {:>8.0} µs {:>8.0} {:>8.0} {:>8.0} {:>6}",
            r.name, r.mean_us, r.min_us, r.max_us, r.stddev_us, r.matching_listings
        );
    }

    // Print comparison with in-memory (conceptual)
    println!("\n=== Comparison Note ===");
    println!("In-memory benchmarks (from search_bench) measure pure Rust functions:");
    println!("  score_listing_100:     ~164 µs (scoring 100 listings against 3 query terms)");
    println!("  compare_search_items:  ~30 ms (comparing 10k listing pairs for sort)");
    println!("  normalize:            ~2 µs (tokenizing 6 query strings)");
    println!();
    println!("Postgres benchmarks measure the full search_listings() pipeline:");
    println!("  SQL query execution (LIKE scan on listings table + JOINs)");
    println!("  Row deserialization (~40 columns per row)");
    println!("  Rust-side sort + scoring (reuses in-memory functions)");
    println!("  Pagination truncation");
    println!();
    println!("Both components matter: PG I/O dominates at low match counts,");
    println!("Rust sort dominates at high match counts.");

    // Cleanup
    sqlx::query("DELETE FROM listings WHERE owner_id LIKE 'pg_seller_%'")
        .execute(&pool)
        .await?;
    eprintln!("\nCleanup: removed seeded listings");

    Ok(())
}
