use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use marketplace_api_contract::{
    Category, Condition, CreateListingRequest, ListingLocation, ListingPayload, ListingStatus,
    ListingSummary, ListingType, Price, SearchRequest, SearchSort,
};
use marketplace_server::repositories::{InMemoryListingRepository, ListingRepository};
use marketplace_server::services::search::SearchService;

fn make_bench_listing(seed: u64) -> ListingSummary {
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

    ListingSummary {
        listing_id: format!("lst_bench_{seed:016x}"),
        version: seed % 10 + 1,
        status: ListingStatus::Active,
        seller_rating: Some((seed as f64 * 0.7).rem_euclid(5.0) + 1.0),
        seller_name: None,
        seller_verified: None,
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: format!("seller_{}", seed % 100),
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

fn bench_score_listing_100(c: &mut Criterion) {
    let listings: Vec<_> = (0..100).map(make_bench_listing).collect();
    let query_terms = vec![
        "macbook".to_string(),
        "pro".to_string(),
        "laptop".to_string(),
    ];

    c.bench_function("score_listing_100", |b| {
        b.iter(|| {
            for listing in &listings {
                black_box(marketplace_server::services::search::score_listing(
                    listing,
                    &query_terms,
                ));
            }
        })
    });
}

fn bench_score_listing_500(c: &mut Criterion) {
    let listings: Vec<_> = (0..500).map(make_bench_listing).collect();
    let query_terms = vec![
        "macbook".to_string(),
        "pro".to_string(),
        "laptop".to_string(),
    ];

    c.bench_function("score_listing_500", |b| {
        b.iter(|| {
            for listing in &listings {
                black_box(marketplace_server::services::search::score_listing(
                    listing,
                    &query_terms,
                ));
            }
        })
    });
}

fn bench_score_listing_1000(c: &mut Criterion) {
    let listings: Vec<_> = (0..1000).map(make_bench_listing).collect();
    let query_terms = vec![
        "macbook".to_string(),
        "pro".to_string(),
        "laptop".to_string(),
    ];

    c.bench_function("score_listing_1000", |b| {
        b.iter(|| {
            for listing in &listings {
                black_box(marketplace_server::services::search::score_listing(
                    listing,
                    &query_terms,
                ));
            }
        })
    });
}

fn bench_compare_search_items_100x100(c: &mut Criterion) {
    let listings: Vec<_> = (0..100).map(make_bench_listing).collect();
    let query_terms = vec!["gaming".to_string(), "pc".to_string()];

    c.bench_function("compare_search_items_100x100", |b| {
        b.iter(|| {
            for a in &listings {
                for b in &listings {
                    black_box(marketplace_server::services::search::compare_search_items(
                        a,
                        b,
                        &query_terms,
                        SearchSort::Relevance,
                    ));
                }
            }
        })
    });
}

fn bench_compare_search_items_500x500(c: &mut Criterion) {
    let listings: Vec<_> = (0..500).map(make_bench_listing).collect();
    let query_terms = vec!["gaming".to_string(), "pc".to_string()];

    c.bench_function("compare_search_items_500x500", |b| {
        b.iter(|| {
            for a in &listings {
                for b in &listings {
                    black_box(marketplace_server::services::search::compare_search_items(
                        a,
                        b,
                        &query_terms,
                        SearchSort::Relevance,
                    ));
                }
            }
        })
    });
}

fn bench_compare_search_items_1000x1000(c: &mut Criterion) {
    let listings: Vec<_> = (0..1000).map(make_bench_listing).collect();
    let query_terms = vec!["gaming".to_string(), "pc".to_string()];

    // Reduced sample size for this expensive benchmark
    let mut group = c.benchmark_group("compare_search_items_1000x1000");
    group.sample_size(30);
    group.bench_function("compare_search_items_1000x1000", |b| {
        b.iter(|| {
            for a in &listings {
                for b in &listings {
                    black_box(marketplace_server::services::search::compare_search_items(
                        a,
                        b,
                        &query_terms,
                        SearchSort::Relevance,
                    ));
                }
            }
        })
    });
    group.finish();
}

// -------------------------------------------------------------------------
// Orchestration benchmarks: SearchService<InMemoryListingRepository> end-to-end
// -------------------------------------------------------------------------

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
        idempotency_key: format!("orch-bench-idem-{seed}"),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: format!("seller_{}", seed % 20),
            listing_type: ListingType::Product,
            category: Some(categories[idx]),
            title: format!("{} #{}", titles[idx], seed),
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

fn make_orchestration_bench(n: u64, c: &mut Criterion, name: &str) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let repo = Arc::new(InMemoryListingRepository::new());

    // Seed listings
    rt.block_on(async {
        for i in 0..n {
            let req = make_bench_listing_request(i);
            repo.insert_listing(&req).await.unwrap();
        }
    });

    let service = SearchService::new(repo);
    let search_request = SearchRequest {
        query: Some("macbook".to_string()),
        sort_by: SearchSort::Relevance,
        limit: Some(20),
        ..Default::default()
    };

    c.bench_function(name, |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(
                    service
                        .search_listings(None, &search_request)
                        .await
                        .unwrap(),
                );
            });
        })
    });
}

fn bench_orchestration_search_100(c: &mut Criterion) {
    make_orchestration_bench(100, c, "orchestration_search_100");
}

fn bench_orchestration_search_500(c: &mut Criterion) {
    make_orchestration_bench(500, c, "orchestration_search_500");
}

fn bench_orchestration_search_1000(c: &mut Criterion) {
    make_orchestration_bench(1000, c, "orchestration_search_1000");
}

fn bench_normalize_search_terms(c: &mut Criterion) {
    let inputs = vec![
        "  MacBook Pro 16-inch M3 Max  ",
        "gaming desktop pc rtx 4090",
        "vintage leather jacket size M",
        "",
        "   ",
        "hello!@# world$%^ &*()",
    ];

    c.bench_function("normalize_search_terms", |b| {
        b.iter(|| {
            for input in &inputs {
                black_box(marketplace_server::services::search::normalize_search_terms(input));
            }
        })
    });
}

criterion_group! {
    name = search;
    config = Criterion::default().sample_size(100);
    targets =
        bench_score_listing_100,
        bench_score_listing_500,
        bench_score_listing_1000,
        bench_compare_search_items_100x100,
        bench_compare_search_items_500x500,
        bench_compare_search_items_1000x1000,
        bench_normalize_search_terms,
        bench_orchestration_search_100,
        bench_orchestration_search_500,
        bench_orchestration_search_1000
}

criterion_main!(search);
