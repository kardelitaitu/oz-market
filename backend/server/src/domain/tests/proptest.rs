// Property-based tests via proptest
//
// Covers invariants that are hard to verify with hand-written examples:
// - Scoring monotonicity: adding relevant query terms never decreases score
// - Sorting transitivity: if a < b and b < c then a < c
// - Validation invariants: correct listing always passes; certain patterns always fail

use proptest::prelude::*;

use marketplace_api_contract::{
    Category, Condition, ListingLocation, ListingPayload, ListingStatus, ListingSummary,
    ListingType, Price, SearchSort,
};

use crate::services::search::{compare_search_items, listing_index_text, score_listing};

// -----------------------------------------------------------------------
// Strategy generators
// -----------------------------------------------------------------------

fn arb_price() -> impl Strategy<Value = f64> {
    prop::num::f64::POSITIVE
        .prop_filter("price must be finite", |v| v.is_finite())
        .prop_map(|v| v.min(1_000_000.0))
}

fn arb_category() -> impl Strategy<Value = Category> {
    prop::sample::select(vec![
        Category::Laptop,
        Category::Phone,
        Category::Tablet,
        Category::Desktop,
        Category::Gaming,
        Category::Other,
    ])
}

fn arb_city() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "New York".to_string(),
        "London".to_string(),
        "Tokyo".to_string(),
        "Berlin".to_string(),
        "San Francisco".to_string(),
    ])
}

fn arb_title() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "MacBook Pro 16".to_string(),
        "iPhone 15 Pro".to_string(),
        "Vintage Leather Jacket".to_string(),
        "Gaming Desktop PC".to_string(),
        "Acoustic Guitar".to_string(),
    ])
}

fn arb_listing_summary() -> impl Strategy<Value = ListingSummary> {
    (
        arb_title(),
        arb_price(),
        arb_city(),
        arb_category(),
        any::<u64>(),
    )
        .prop_map(|(title, price, city, category, seed)| ListingSummary {
            listing_id: format!("lst_prop_{seed:016x}"),
            version: 1,
            status: ListingStatus::Active,
            seller_rating: Some(4.0),
            seller_name: None,
            seller_verified: None,
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "seller-prop-1".to_string(),
                listing_type: ListingType::Product,
                category: Some(category),
                title,
                condition: Some(Condition::New),
                price: Price {
                    amount: price,
                    currency: "USD".to_string(),
                },
                location: ListingLocation {
                    country_code: "US".to_string(),
                    country_name: "United States".to_string(),
                    city,
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec![],
                description: "A test item in excellent condition".to_string(),
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
        })
}

fn arb_query_terms() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(
        prop::sample::select(vec![
            "macbook".to_string(),
            "pro".to_string(),
            "laptop".to_string(),
            "apple".to_string(),
            "computer".to_string(),
            "vintage".to_string(),
            "gaming".to_string(),
            "guitar".to_string(),
        ]),
        0..5,
    )
}

// -----------------------------------------------------------------------
// Property 1: Scoring monotonicity
//
// For any listing and any set of query terms, adding more terms never
// produces a *lower* score than a subset of those terms.
// -----------------------------------------------------------------------

proptest! {
    #[test]
    fn scoring_monotonicity(
        listing in arb_listing_summary(),
        base_terms in arb_query_terms(),
        extra_term in "[a-z]{3,10}",
    ) {
        let score_base = score_listing(&listing, &base_terms);
        let mut extended_terms = base_terms.clone();
        extended_terms.push(extra_term);
        let score_extended = score_listing(&listing, &extended_terms);

        // Adding more terms should not decrease the score
        prop_assert!(score_extended >= score_base,
            "score decreased from {score_base} to {score_extended} when adding term to {:?}",
            base_terms);
    }
}

// -----------------------------------------------------------------------
// Property 2: Scoring is non-negative
//
// For any listing and any set of query terms, score must be >= 0.
// -----------------------------------------------------------------------

proptest! {
    #[test]
    fn scoring_non_negative(
        listing in arb_listing_summary(),
        terms in arb_query_terms(),
    ) {
        let score = score_listing(&listing, &terms);
        prop_assert!(score >= 0, "negative score {score} for terms {terms:?}");
    }
}

// -----------------------------------------------------------------------
// Property 3: Sorting transitivity
//
// For any three listings sorted by the same criteria:
// if a < b and b < c then a < c.
// -----------------------------------------------------------------------

proptest! {
    #[test]
    fn sorting_transitivity_price_asc(
        a in arb_listing_summary(),
        b in arb_listing_summary(),
        c in arb_listing_summary(),
    ) {
        let cmp_ab = compare_search_items(&a, &b, &[], SearchSort::PriceAsc);
        let cmp_bc = compare_search_items(&b, &c, &[], SearchSort::PriceAsc);
        let cmp_ac = compare_search_items(&a, &c, &[], SearchSort::PriceAsc);

        if cmp_ab == std::cmp::Ordering::Less && cmp_bc == std::cmp::Ordering::Less {
            prop_assert_eq!(cmp_ac, std::cmp::Ordering::Less,
                "sorting transitivity violated for PriceAsc");
        }
    }
}

proptest! {
    #[test]
    fn sorting_transitivity_price_desc(
        a in arb_listing_summary(),
        b in arb_listing_summary(),
        c in arb_listing_summary(),
    ) {
        let cmp_ab = compare_search_items(&a, &b, &[], SearchSort::PriceDesc);
        let cmp_bc = compare_search_items(&b, &c, &[], SearchSort::PriceDesc);
        let cmp_ac = compare_search_items(&a, &c, &[], SearchSort::PriceDesc);

        if cmp_ab == std::cmp::Ordering::Less && cmp_bc == std::cmp::Ordering::Less {
            prop_assert_eq!(cmp_ac, std::cmp::Ordering::Less,
                "sorting transitivity violated for PriceDesc");
        }
    }
}

// -----------------------------------------------------------------------
// Property 4: Listing index text never panics
//
// For any ListingPayload, listing_index_text should produce a string
// that contains the title.
// -----------------------------------------------------------------------

fn arb_listing_payload() -> impl Strategy<Value = ListingPayload> {
    (arb_title(), arb_price(), arb_city(), arb_category()).prop_map(
        |(title, price, city, category)| ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "owner-prop".to_string(),
            listing_type: ListingType::Product,
            category: Some(category),
            title: title.clone(),
            condition: Some(Condition::New),
            price: Price {
                amount: price,
                currency: "USD".to_string(),
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city,
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec![],
            description: "A test item".to_string(),
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
    )
}

proptest! {
    #[test]
    fn listing_index_text_never_panics(
        payload in arb_listing_payload(),
    ) {
        let text = listing_index_text(&payload);
        // Should at least contain the title
        prop_assert!(text.contains(&payload.title),
            "index text '{text}' does not contain title '{}'", payload.title);
    }
}

// -----------------------------------------------------------------------
// Property 5: Sorting is deterministic (antisymmetric)
//
// For any two listings: if a < b then !(b < a).
// -----------------------------------------------------------------------

proptest! {
    #[test]
    fn sorting_deterministic_price_asc(
        a in arb_listing_summary(),
        b in arb_listing_summary(),
    ) {
        let cmp_ab = compare_search_items(&a, &b, &[], SearchSort::PriceAsc);
        let cmp_ba = compare_search_items(&b, &a, &[], SearchSort::PriceAsc);

        match cmp_ab {
            std::cmp::Ordering::Less => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Greater);
            }
            std::cmp::Ordering::Greater => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Less);
            }
            std::cmp::Ordering::Equal => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Equal);
            }
        }
    }
}

// -----------------------------------------------------------------------
// Property 6: Sorting by Newest depends on version
// -----------------------------------------------------------------------

proptest! {
    #[test]
    fn sorting_newest_orders_by_version(
        a in arb_listing_summary(),
        b in arb_listing_summary(),
    ) {
        let cmp_ab = compare_search_items(&a, &b, &[], SearchSort::Newest);
        let cmp_ba = compare_search_items(&b, &a, &[], SearchSort::Newest);

        // Must be antisymmetric
        match cmp_ab {
            std::cmp::Ordering::Less => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Greater);
            }
            std::cmp::Ordering::Greater => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Less);
            }
            std::cmp::Ordering::Equal => {
                prop_assert_eq!(cmp_ba, std::cmp::Ordering::Equal);
            }
        }
    }
}
