//! Additional serde/edge-case coverage for the frozen V1 API contract types.
//! Pinned here so future spec changes that touch the wire format show up as
//! failing tests rather than silent regressions.

use oz_market_api_contract::*;

// ---------------------------------------------------------------------
// Enum variant coverage
// ---------------------------------------------------------------------

#[test]
fn negotiation_status_all_variants_serde() {
    let variants = [
        (NegotiationStatus::Open, "open"),
        (NegotiationStatus::Countered, "countered"),
        (NegotiationStatus::NearClose, "near_close"),
        (NegotiationStatus::Reserved, "reserved"),
        (NegotiationStatus::ContactRequested, "contact_requested"),
        (NegotiationStatus::ContactRevealed, "contact_revealed"),
        (NegotiationStatus::Closed, "closed"),
        (NegotiationStatus::Cancelled, "cancelled"),
    ];
    for (variant, expected) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let back: NegotiationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, variant);
    }
}

#[test]
fn contact_reveal_status_all_variants_serde() {
    let variants = [
        (ContactRevealStatus::Pending, "pending"),
        (ContactRevealStatus::Approved, "approved"),
        (ContactRevealStatus::Rejected, "rejected"),
        (ContactRevealStatus::Expired, "expired"),
    ];
    for (variant, expected) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let back: ContactRevealStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, variant);
    }
}

#[test]
fn negotiation_history_entry_type_all_variants_serde() {
    let variants = [
        (NegotiationHistoryEntryType::Offer, "offer"),
        (NegotiationHistoryEntryType::Accept, "accept"),
        (NegotiationHistoryEntryType::Reject, "reject"),
    ];
    for (variant, expected) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let back: NegotiationHistoryEntryType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, variant);
    }
}

#[test]
fn api_error_code_all_variants_serde() {
    let variants = [
        (ApiErrorCode::InvalidField, "invalid_field"),
        (ApiErrorCode::MissingField, "missing_field"),
        (ApiErrorCode::Conflict, "conflict"),
        (ApiErrorCode::NotFound, "not_found"),
        (ApiErrorCode::RateLimited, "rate_limited"),
        (ApiErrorCode::Unauthorized, "unauthorized"),
        (ApiErrorCode::Forbidden, "forbidden"),
        (ApiErrorCode::OwnerMismatch, "owner_mismatch"),
        (ApiErrorCode::CredentialRevoked, "credential_revoked"),
        (ApiErrorCode::QuotaExceeded, "quota_exceeded"),
        (ApiErrorCode::TrustReviewRequired, "trust_review_required"),
        (ApiErrorCode::ReservationConflict, "reservation_conflict"),
        (ApiErrorCode::VersionConflict, "version_conflict"),
        (ApiErrorCode::InvalidTransition, "invalid_transition"),
    ];
    for (variant, expected) in variants {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let back: ApiErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, variant);
    }
}

// ---------------------------------------------------------------------
// Agent contract (spec 0014)
// ---------------------------------------------------------------------

#[test]
fn agent_query_request_roundtrip() {
    let req = AgentQueryRequest {
        query: "find me a laptop under $1000".to_string(),
        conversation_id: Some("conv-001".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["query"], "find me a laptop under $1000");
    assert_eq!(json["conversation_id"], "conv-001");
    let back: AgentQueryRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn agent_query_request_omits_optional_conversation_id() {
    let req = AgentQueryRequest {
        query: "hi".to_string(),
        conversation_id: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("conversation_id").is_none());
}

#[test]
fn agent_query_response_roundtrip() {
    let resp = AgentQueryResponse {
        message: "Here are some matches".to_string(),
        actions: vec![AgentAction {
            action_type: "open_listing".to_string(),
            label: "View listing".to_string(),
            params: serde_json::json!({"listing_id": "lst_1"}),
        }],
        conversation_id: "conv-001".to_string(),
        listing_ids: Some(vec!["lst_1".to_string(), "lst_2".to_string()]),
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["message"], "Here are some matches");
    assert_eq!(json["actions"].as_array().unwrap().len(), 1);
    assert_eq!(json["listing_ids"].as_array().unwrap().len(), 2);
    let back: AgentQueryResponse = serde_json::from_value(json).unwrap();
    assert_eq!(back, resp);
}

#[test]
fn agent_query_response_omits_empty_actions_and_listing_ids() {
    let resp = AgentQueryResponse {
        message: "no results".to_string(),
        actions: vec![],
        conversation_id: "conv-001".to_string(),
        listing_ids: None,
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert!(json.get("actions").is_none());
    assert!(json.get("listing_ids").is_none());
}

// ---------------------------------------------------------------------
// Negotiation request payloads (one roundtrip each)
// ---------------------------------------------------------------------

#[test]
fn submit_offer_request_roundtrip() {
    let req = SubmitOfferRequest {
        offer_currency: "USD".to_string(),
        offer_amount: 800.0,
        idempotency_key: "idem-1".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["offer_amount"], 800.0);
    let back: SubmitOfferRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn accept_negotiation_request_roundtrip() {
    let req = AcceptNegotiationRequest {
        idempotency_key: "idem-accept-1".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["idempotency_key"], "idem-accept-1");
    let back: AcceptNegotiationRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn reject_negotiation_request_roundtrip() {
    let req = RejectNegotiationRequest {
        idempotency_key: "idem-reject-1".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["idempotency_key"], "idem-reject-1");
    let back: RejectNegotiationRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn request_contact_reveal_request_roundtrip() {
    let req = RequestContactRevealRequest {
        idempotency_key: "idem-reveal-1".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["idempotency_key"], "idem-reveal-1");
    let back: RequestContactRevealRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back, req);
}

#[test]
fn negotiation_history_entry_roundtrip() {
    let entry = NegotiationHistoryEntry {
        entry_id: "entry_1".to_string(),
        entry_type: NegotiationHistoryEntryType::Offer,
        offer_currency: "USD".to_string(),
        offer_amount: 800.0,
        actor_subject: "agent_buy_1".to_string(),
        actor_role: "buyer_negotiator".to_string(),
        idempotency_key: "idem-offer-1".to_string(),
        resulting_status: NegotiationStatus::Countered,
        created_at: "2026-05-10T12:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["entry_type"], "offer");
    assert_eq!(json["resulting_status"], "countered");
    let back: NegotiationHistoryEntry = serde_json::from_value(json).unwrap();
    assert_eq!(back, entry);
}

// ---------------------------------------------------------------------
// Error envelope edge cases
// ---------------------------------------------------------------------

#[test]
fn api_error_response_omits_field_when_none() {
    let err = ApiErrorResponse {
        error: ApiErrorDetail {
            code: ApiErrorCode::Forbidden,
            message: "not allowed".to_string(),
            field: None,
        },
    };
    let json = serde_json::to_value(&err).unwrap();
    assert!(json["error"].get("field").is_none());
    assert_eq!(json["error"]["code"], "forbidden");
}

#[test]
fn api_error_response_rejects_unknown_code() {
    let json = serde_json::json!({
        "error": {
            "code": "not_a_real_code",
            "message": "x"
        }
    });
    let result: Result<ApiErrorResponse, _> = serde_json::from_value(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------
// ListingSummary minimal-roundtrip
// ---------------------------------------------------------------------

fn minimal_listing_payload() -> ListingPayload {
    ListingPayload {
        schema_version: "1.0.0".to_string(),
        owner_id: "u".to_string(),
        listing_type: ListingType::Product,
        category: Some(Category::Laptop),
        title: "t".to_string(),
        condition: Some(Condition::Used),
        price: Price {
            currency: "USD".to_string(),
            amount: 1.0,
        },
        location: ListingLocation {
            country_code: "US".to_string(),
            country_name: "US".to_string(),
            city: "SF".to_string(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        },
        picture_urls: vec![],
        description: "d".to_string(),
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
    }
}

#[test]
fn listing_summary_roundtrip_with_and_without_seller_fields() {
    let with_seller = ListingSummary {
        listing_id: "lst_1".to_string(),
        status: ListingStatus::Active,
        version: 1,
        listing: minimal_listing_payload(),
        seller_name: Some("TechStore".to_string()),
        seller_rating: Some(4.8),
        seller_verified: Some(true),
    };
    let json = serde_json::to_value(&with_seller).unwrap();
    assert_eq!(json["seller_name"], "TechStore");
    assert_eq!(json["seller_rating"], 4.8);
    assert!(json["seller_verified"].as_bool().unwrap());
    let back: ListingSummary = serde_json::from_value(json).unwrap();
    assert_eq!(back, with_seller);

    let without_seller = ListingSummary {
        listing_id: "lst_2".to_string(),
        status: ListingStatus::Active,
        version: 1,
        listing: minimal_listing_payload(),
        seller_name: None,
        seller_rating: None,
        seller_verified: None,
    };
    let json = serde_json::to_value(&without_seller).unwrap();
    assert!(json.get("seller_name").is_none());
    assert!(json.get("seller_rating").is_none());
    assert!(json.get("seller_verified").is_none());
}

#[test]
fn shipping_info_roundtrip() {
    let info = ShippingInfo {
        local_pickup: true,
        shipping_available: false,
        shipping_cost: None,
        shipping_regions: Some(vec!["US".to_string(), "CA".to_string()]),
    };
    let json = serde_json::to_value(&info).unwrap();
    assert!(json["local_pickup"].as_bool().unwrap());
    assert!(!json["shipping_available"].as_bool().unwrap());
    assert!(json.get("shipping_cost").is_none());
    let back: ShippingInfo = serde_json::from_value(json).unwrap();
    assert_eq!(back, info);
}
