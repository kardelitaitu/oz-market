use oz_market_api_contract::*;

#[test]
fn category_enum_serde_roundtrip() {
    let variants = [
        (Category::Laptop, "laptop"),
        (Category::Phone, "phone"),
        (Category::Tablet, "tablet"),
        (Category::Desktop, "desktop"),
        (Category::Monitor, "monitor"),
        (Category::Accessory, "accessory"),
        (Category::Camera, "camera"),
        (Category::Audio, "audio"),
        (Category::Gaming, "gaming"),
        (Category::Appliance, "appliance"),
        (Category::Furniture, "furniture"),
        (Category::VehiclePart, "vehicle_part"),
        (Category::Other, "other"),
    ];
    for (variant, expected_str) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{}\"", expected_str));
        let decoded: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, variant);
    }
}

#[test]
fn listing_status_enum_serde_roundtrip() {
    let variants = [
        (ListingStatus::Draft, "draft"),
        (ListingStatus::Active, "active"),
        (ListingStatus::Reserved, "reserved"),
        (ListingStatus::Sold, "sold"),
        (ListingStatus::Archived, "archived"),
    ];
    for (variant, expected_str) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{}\"", expected_str));
        let decoded: ListingStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, variant);
    }
}

#[test]
fn search_sort_enum_serde_roundtrip() {
    let variants = [
        (SearchSort::Relevance, "relevance"),
        (SearchSort::Newest, "newest"),
        (SearchSort::PriceAsc, "price_asc"),
        (SearchSort::PriceDesc, "price_desc"),
        (SearchSort::RatingHighest, "rating_highest"),
        (SearchSort::RatingLowest, "rating_lowest"),
        (SearchSort::PricePerSqmAsc, "price_per_sqm_asc"),
        (SearchSort::PricePerSqmDesc, "price_per_sqm_desc"),
    ];
    for (variant, expected_str) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{}\"", expected_str));
        let decoded: SearchSort = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, variant);
    }
}

#[test]
fn search_request_default_values() {
    let req = SearchRequest::default();
    assert_eq!(req.sort_by, SearchSort::Relevance);
    assert!(req.query.is_none());
    assert!(req.category.is_none());
    assert!(req.listing_type.is_none());
    assert!(req.service_type.is_none());
    assert!(req.property_transaction_type.is_none());
    assert!(req.is_near_me.is_none());
}

#[test]
fn full_search_request_serde_roundtrip() {
    let req = SearchRequest {
        query: Some("laptop".into()),
        category: Some(Category::Laptop),
        condition: Some(Condition::Used),
        price: Some(SearchPriceFilter {
            currency: Some("USD".into()),
            min_amount: Some(500.0),
            max_amount: Some(1500.0),
        }),
        location: Some(SearchLocationFilter {
            country_code: Some("US".into()),
            city: Some("San Francisco".into()),
        }),
        status: Some(ListingStatus::Active),
        min_seller_rating: Some(4.5),
        is_verified_seller_only: Some(true),
        listing_type: Some(ListingType::Product),
        service_type: Some(ServiceType::Online),
        property_transaction_type: Some(PropertyTransactionType::Rent),
        property_sub_type: Some(PropertySubType::Apartment),
        min_bedrooms: Some(2),
        min_bathrooms: Some(1),
        min_area_sqm: Some(50.0),
        max_area_sqm: Some(200.0),
        sort_by: SearchSort::PriceAsc,
        limit: Some(10),
        cursor: Some("abc123".into()),
        is_near_me: Some(true),
        user_latitude: Some(37.7749),
        user_longitude: Some(-122.4194),
        radius_km: Some(10.0),
        owner_id: None,
    };
    let json = serde_json::to_value(&req).expect("serialization failed");
    let deserialized: SearchRequest = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(req, deserialized);
}

#[test]
fn search_request_optional_fields_omitted_when_none() {
    let req = SearchRequest {
        query: Some("phone".into()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).expect("serialization failed");
    assert!(json.get("category").is_none());
    assert!(json.get("price").is_none());
    assert!(json.get("is_near_me").is_none());
    assert!(json.get("service_type").is_none());
    assert!(json.get("property_transaction_type").is_none());
}

#[test]
fn listing_type_enum_serde_roundtrip() {
    assert_eq!(
        serde_json::to_string(&ListingType::Product).unwrap(),
        "\"product\""
    );
    assert_eq!(
        serde_json::to_string(&ListingType::Service).unwrap(),
        "\"service\""
    );
    assert_eq!(
        serde_json::to_string(&ListingType::Property).unwrap(),
        "\"property\""
    );
}

#[test]
fn service_type_enum_serde_roundtrip() {
    assert_eq!(
        serde_json::to_string(&ServiceType::Local).unwrap(),
        "\"local\""
    );
    assert_eq!(
        serde_json::to_string(&ServiceType::Online).unwrap(),
        "\"online\""
    );
}

#[test]
fn property_transaction_type_enum_serde_roundtrip() {
    assert_eq!(
        serde_json::to_string(&PropertyTransactionType::Rent).unwrap(),
        "\"rent\""
    );
    assert_eq!(
        serde_json::to_string(&PropertyTransactionType::Sale).unwrap(),
        "\"sale\""
    );
}

#[test]
fn property_sub_type_enum_serde_roundtrip() {
    assert_eq!(
        serde_json::to_string(&PropertySubType::Building).unwrap(),
        "\"building\""
    );
    assert_eq!(
        serde_json::to_string(&PropertySubType::House).unwrap(),
        "\"house\""
    );
    assert_eq!(
        serde_json::to_string(&PropertySubType::Apartment).unwrap(),
        "\"apartment\""
    );
    assert_eq!(
        serde_json::to_string(&PropertySubType::Land).unwrap(),
        "\"land\""
    );
}

#[test]
fn condition_enum_serde_roundtrip() {
    assert_eq!(serde_json::to_string(&Condition::New).unwrap(), "\"new\"");
    assert_eq!(serde_json::to_string(&Condition::Used).unwrap(), "\"used\"");
    assert_eq!(
        serde_json::to_string(&Condition::Refurbished).unwrap(),
        "\"refurbished\""
    );
}

#[test]
fn price_struct_serde_roundtrip() {
    let price = Price {
        currency: "USD".into(),
        amount: 1299.99,
    };
    let json = serde_json::to_value(&price).unwrap();
    assert_eq!(json["currency"], "USD");
    assert_eq!(json["amount"], 1299.99);
    let decoded: Price = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, price);
}

#[test]
fn listing_location_serde_roundtrip_with_geolocation() {
    let location = ListingLocation {
        country_code: "US".into(),
        country_name: "United States".into(),
        city: "San Francisco".into(),
        latitude: Some(37.7749),
        longitude: Some(-122.4194),
        geolocation_opt_out: Some(false),
    };
    let json = serde_json::to_value(&location).unwrap();
    assert_eq!(json["latitude"], 37.7749);
    assert_eq!(json["longitude"], -122.4194);
    let decoded: ListingLocation = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, location);
}

#[test]
fn listing_location_omits_geolocation_when_none() {
    let location = ListingLocation {
        country_code: "DE".into(),
        country_name: "Germany".into(),
        city: "Berlin".into(),
        latitude: None,
        longitude: None,
        geolocation_opt_out: None,
    };
    let json = serde_json::to_value(&location).unwrap();
    assert!(json.get("latitude").is_none());
    assert!(json.get("longitude").is_none());
}

#[test]
fn listing_payload_product_roundtrip() {
    let payload = ListingPayload {
        schema_version: "1.0.0".into(),
        owner_id: "user_123".into(),
        listing_type: ListingType::Product,
        category: Some(Category::Laptop),
        title: "MacBook Pro".into(),
        condition: Some(Condition::Used),
        price: Price {
            currency: "USD".into(),
            amount: 1299.99,
        },
        location: ListingLocation {
            country_code: "US".into(),
            country_name: "United States".into(),
            city: "San Francisco".into(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "A used MacBook".into(),
        attributes: None,
        sku: Some("MBP-14-2024".into()),
        quantity: Some(1),
        shipping_info: None,
        condition_details: Some("Minor scuff".into()),
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
    };
    let json = serde_json::to_value(&payload).expect("serialization failed");
    assert_eq!(json["schema_version"], "1.0.0");
    assert_eq!(json["listing_type"], "product");
    assert_eq!(json["price"]["amount"], 1299.99);
    let decoded: ListingPayload = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, payload);
}

#[test]
fn listing_payload_service_roundtrip() {
    let payload = ListingPayload {
        schema_version: "1.0.0".into(),
        owner_id: "consultant_01".into(),
        listing_type: ListingType::Service,
        category: None,
        title: "Web Dev Consultation".into(),
        condition: None,
        price: Price {
            currency: "EUR".into(),
            amount: 150.0,
        },
        location: ListingLocation {
            country_code: "DE".into(),
            country_name: "Germany".into(),
            city: "Berlin".into(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "1-hour remote consultation".into(),
        attributes: None,
        sku: None,
        quantity: None,
        shipping_info: None,
        condition_details: None,
        seller_notes: None,
        service_type: Some(ServiceType::Online),
        hourly_rate: Some(150.0),
        project_rate: Some(1200.0),
        qualifications: Some(vec!["AWS Certified".into(), "Rust Expert".into()]),
        service_radius_km: Some(50),
        property_transaction_type: None,
        property_sub_type: None,
        area_sqm: None,
        bedrooms: None,
        bathrooms: None,
        year_built: None,
        lot_size_sqm: None,
        zoning: None,
    };
    let json = serde_json::to_value(&payload).expect("serialization failed");
    assert_eq!(json["service_type"], "online");
    assert_eq!(json["hourly_rate"], 150.0);
    assert_eq!(json["qualifications"].as_array().unwrap().len(), 2);
    let decoded: ListingPayload = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, payload);
}

#[test]
fn listing_payload_property_roundtrip() {
    let payload = ListingPayload {
        schema_version: "1.0.0".into(),
        owner_id: "landlord_42".into(),
        listing_type: ListingType::Property,
        category: None,
        title: "Downtown Apartment".into(),
        condition: None,
        price: Price {
            currency: "GBP".into(),
            amount: 1800.0,
        },
        location: ListingLocation {
            country_code: "GB".into(),
            country_name: "United Kingdom".into(),
            city: "London".into(),
            latitude: Some(51.5074),
            longitude: Some(-0.1278),
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "2-bed apartment".into(),
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
        property_transaction_type: Some(PropertyTransactionType::Rent),
        property_sub_type: Some(PropertySubType::Apartment),
        area_sqm: Some(75.0),
        bedrooms: Some(2),
        bathrooms: Some(1),
        year_built: Some(2010),
        lot_size_sqm: None,
        zoning: Some("Residential".into()),
    };
    let json = serde_json::to_value(&payload).expect("serialization failed");
    assert_eq!(json["property_transaction_type"], "rent");
    assert_eq!(json["bedrooms"], 2);
    let decoded: ListingPayload = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, payload);
}

#[test]
fn create_listing_request_roundtrip() {
    let req = CreateListingRequest {
        idempotency_key: "req-abc-123".into(),
        listing: ListingPayload {
            schema_version: "1.0.0".into(),
            owner_id: "user_123".into(),
            listing_type: ListingType::Product,
            category: Some(Category::Phone),
            title: "iPhone 15".into(),
            condition: Some(Condition::New),
            price: Price {
                currency: "USD".into(),
                amount: 999.0,
            },
            location: ListingLocation {
                country_code: "US".into(),
                country_name: "United States".into(),
                city: "New York".into(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec![],
            description: "Brand new iPhone".into(),
            attributes: None,
            sku: Some("IP15-128".into()),
            quantity: Some(1),
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
    };
    let json = serde_json::to_value(&req).expect("serialization failed");
    assert_eq!(json["idempotency_key"], "req-abc-123");
    assert_eq!(json["listing"]["title"], "iPhone 15");
    let decoded: CreateListingRequest =
        serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, req);
}

#[test]
fn search_response_roundtrip() {
    let response = SearchResponse {
        items: vec![ListingSummary {
            listing_id: "list_001".into(),
            status: ListingStatus::Active,
            version: 1,
            listing: ListingPayload {
                schema_version: "1.0.0".into(),
                owner_id: "seller_1".into(),
                listing_type: ListingType::Product,
                category: Some(Category::Laptop),
                title: "ThinkPad X1".into(),
                condition: Some(Condition::Refurbished),
                price: Price {
                    currency: "USD".into(),
                    amount: 899.0,
                },
                location: ListingLocation {
                    country_code: "US".into(),
                    country_name: "United States".into(),
                    city: "Chicago".into(),
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec![],
                description: "Refurbished ThinkPad".into(),
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
            seller_name: Some("TechStore".into()),
            seller_rating: Some(4.8),
            seller_verified: Some(true),
        }],
        applied_sort_by: SearchSort::Relevance,
        next_cursor: Some("cursor_abc".into()),
    };
    let json = serde_json::to_value(&response).expect("serialization failed");
    assert_eq!(json["items"].as_array().unwrap().len(), 1);
    let decoded: SearchResponse = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded.items.len(), 1);
    assert_eq!(decoded.next_cursor, Some("cursor_abc".into()));
}

#[test]
fn open_negotiation_request_roundtrip() {
    let req = OpenNegotiationRequest {
        listing_id: "list_001".into(),
        buyer_agent_id: "agent_buy_1".into(),
        offer_currency: "USD".into(),
        offer_amount: 850.0,
        idempotency_key: "neg-req-001".into(),
    };
    let json = serde_json::to_value(&req).expect("serialization failed");
    assert_eq!(json["listing_id"], "list_001");
    assert_eq!(json["offer_amount"], 850.0);
    let decoded: OpenNegotiationRequest =
        serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, req);
}

#[test]
fn negotiation_response_roundtrip() {
    let resp = NegotiationResponse {
        negotiation_id: "neg_001".into(),
        listing_id: "list_001".into(),
        buyer_agent_id: "agent_buy_1".into(),
        status: NegotiationStatus::Open,
        offer_currency: "USD".into(),
        latest_offer_amount: 850.0,
        reservation_lease_id: Some("lease_abc".into()),
        final_offer_amount: None,
        reveal_id: None,
        offer_history: vec![],
        version: 1,
        updated_at: "2026-05-10T12:00:00Z".into(),
    };
    let json = serde_json::to_value(&resp).expect("serialization failed");
    assert_eq!(json["status"], "open");
    assert_eq!(json["reservation_lease_id"], "lease_abc");
    let decoded: NegotiationResponse =
        serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, resp);
}

#[test]
fn contact_reveal_response_roundtrip() {
    let resp = ContactRevealResponse {
        reveal_id: "rev_001".into(),
        negotiation_id: "neg_001".into(),
        reveal_status: ContactRevealStatus::Approved,
        revealed_phone_reference: Some("+1-555-0100".into()),
        expires_at: Some("2026-05-11T12:00:00Z".into()),
        approved_at: Some("2026-05-10T12:05:00Z".into()),
        updated_at: "2026-05-10T12:05:00Z".into(),
    };
    let json = serde_json::to_value(&resp).expect("serialization failed");
    assert_eq!(json["reveal_status"], "approved");
    assert_eq!(json["revealed_phone_reference"], "+1-555-0100");
    let decoded: ContactRevealResponse =
        serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, resp);
}

#[test]
fn api_error_response_roundtrip() {
    let err = ApiErrorResponse {
        error: ApiErrorDetail {
            code: ApiErrorCode::NotFound,
            message: "Listing not found".into(),
            field: Some("listing_id".into()),
        },
    };
    let json = serde_json::to_value(&err).expect("serialization failed");
    assert_eq!(json["error"]["code"], "not_found");
    assert_eq!(json["error"]["field"], "listing_id");
    let decoded: ApiErrorResponse = serde_json::from_value(json).expect("deserialization failed");
    assert_eq!(decoded, err);
}

#[test]
fn listing_payload_omits_optional_none_fields() {
    let payload = ListingPayload {
        schema_version: "1.0.0".into(),
        owner_id: "user_123".into(),
        listing_type: ListingType::Service,
        category: None,
        title: "Consulting".into(),
        condition: None,
        price: Price {
            currency: "USD".into(),
            amount: 100.0,
        },
        location: ListingLocation {
            country_code: "US".into(),
            country_name: "United States".into(),
            city: "Remote".into(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "Remote consulting".into(),
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
    };
    let json = serde_json::to_value(&payload).unwrap();
    assert!(json.get("category").is_none());
    assert!(json.get("condition").is_none());
    assert!(json.get("sku").is_none());
    assert!(json.get("shipping_info").is_none());
    assert!(json.get("service_type").is_none());
    assert!(json.get("property_transaction_type").is_none());
}
