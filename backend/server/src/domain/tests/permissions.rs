// Permission business rule tests
//
// Covers all authorize_* functions in auth/mod.rs:
// - authorize_create_listing    — seller CRUD
// - authorize_get_listing       — public read
// - authorize_search_listings   — public search
// - authorize_open_negotiation  — buyer context
// - authorize_request_contact_reveal — participant check
// - authorize_approve_contact_reveal — seller/owner check
// - authorize (generic)         — scope, role, ownership, admin bypass

use crate::auth::{
    authorize, authorize_approve_contact_reveal, authorize_create_listing, authorize_get_listing,
    authorize_open_negotiation, authorize_request_contact_reveal, authorize_search_listings,
    Action, AuthzError, AuthzErrorKind, OwnershipContext,
};
use oz_market_auth_core::{Claims, Role, Scope};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn seller_claims() -> Claims {
    Claims {
        sub: "seller-1".into(),
        roles: vec![
            Role::SellerListingWriter,
            Role::SellerNegotiator,
            Role::SellerContactRevealApprover,
        ],
        scopes: vec![
            Scope::ListingCreate,
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationOfferSubmit,
            Scope::NegotiationRevealRequest,
            Scope::RevealApprove,
        ],
        seller_account_id: Some("seller-1".into()),
        buyer_agent_id: Some("buyer-1".into()),
        hardware_id: None,
        exp: None,
    }
}

fn buyer_claims() -> Claims {
    Claims {
        sub: "buyer-1".into(),
        roles: vec![Role::BuyerSearcher, Role::BuyerNegotiator],
        scopes: vec![
            Scope::ListingSearch,
            Scope::ListingRead,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationOfferSubmit,
            Scope::NegotiationRevealRequest,
        ],
        seller_account_id: None,
        buyer_agent_id: Some("buyer-1".into()),
        hardware_id: None,
        exp: None,
    }
}

fn admin_claims() -> Claims {
    Claims {
        sub: "admin-1".into(),
        roles: vec![Role::Admin],
        scopes: vec![
            Scope::ListingCreate,
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationOfferSubmit,
            Scope::NegotiationRevealRequest,
            Scope::RevealApprove,
        ],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

fn support_claims() -> Claims {
    Claims {
        sub: "support-1".into(),
        roles: vec![Role::SupportReviewer],
        scopes: vec![
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationRead,
        ],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

fn claims_with_role_and_scopes(sub: &str, roles: Vec<Role>, scopes: Vec<Scope>) -> Claims {
    Claims {
        sub: sub.into(),
        roles,
        scopes,
        seller_account_id: Some("seller-1".into()),
        buyer_agent_id: Some("buyer-1".into()),
        hardware_id: None,
        exp: None,
    }
}

// -----------------------------------------------------------------------
// 1. authorize_create_listing — seller CRUD
// -----------------------------------------------------------------------

#[test]
fn create_listing_allows_matching_seller_writer() {
    assert!(authorize_create_listing(&seller_claims(), "seller-1").is_ok());
}

#[test]
fn create_listing_rejects_wrong_owner() {
    let err = authorize_create_listing(&seller_claims(), "seller-999").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn create_listing_allows_admin_any_owner() {
    assert!(authorize_create_listing(&admin_claims(), "any-seller").is_ok());
}

#[test]
fn create_listing_rejects_missing_scope() {
    let claims = claims_with_role_and_scopes("seller-1", vec![Role::SellerListingWriter], vec![]);
    let err = authorize_create_listing(&claims, "seller-1").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

#[test]
fn create_listing_rejects_buyer_role() {
    let claims = claims_with_role_and_scopes(
        "buyer-1",
        vec![Role::BuyerNegotiator],
        vec![Scope::ListingCreate],
    );
    let err = authorize_create_listing(&claims, "buyer-1").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingRole);
}

// -----------------------------------------------------------------------
// 2. authorize_get_listing — public read
// -----------------------------------------------------------------------

#[test]
fn get_listing_allows_no_claims() {
    // authorize_get_listing accepts Option<&Claims>
    assert!(authorize_get_listing(None).is_ok());
}

#[test]
fn get_listing_allows_any_claims() {
    assert!(authorize_get_listing(Some(&buyer_claims())).is_ok());
    assert!(authorize_get_listing(Some(&seller_claims())).is_ok());
    assert!(authorize_get_listing(Some(&support_claims())).is_ok());
}

// -----------------------------------------------------------------------
// 3. authorize_search_listings — public search
// -----------------------------------------------------------------------

#[test]
fn search_listings_allows_no_claims() {
    assert!(authorize_search_listings(None).is_ok());
}

#[test]
fn search_listings_allows_any_claims() {
    assert!(authorize_search_listings(Some(&buyer_claims())).is_ok());
    assert!(authorize_search_listings(Some(&seller_claims())).is_ok());
    assert!(authorize_search_listings(Some(&support_claims())).is_ok());
}

// -----------------------------------------------------------------------
// 4. authorize_open_negotiation — buyer context
// -----------------------------------------------------------------------

#[test]
fn open_negotiation_allows_matching_buyer() {
    assert!(authorize_open_negotiation(&buyer_claims(), "buyer-1").is_ok());
}

#[test]
fn open_negotiation_rejects_wrong_buyer() {
    let err = authorize_open_negotiation(&buyer_claims(), "buyer-999").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn open_negotiation_allows_admin_any_buyer() {
    assert!(authorize_open_negotiation(&admin_claims(), "any-buyer").is_ok());
}

#[test]
fn open_negotiation_rejects_seller_role_without_buyer_scopes() {
    let claims = claims_with_role_and_scopes(
        "seller-1",
        vec![Role::SellerListingWriter, Role::BuyerNegotiator],
        vec![Scope::ListingCreate, Scope::ListingRead],
    );
    let err = authorize_open_negotiation(&claims, "seller-1").unwrap_err();
    // Scope check happens before role check — MissingScope is correct
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

#[test]
fn open_negotiation_rejects_missing_scope() {
    let claims = claims_with_role_and_scopes(
        "buyer-1",
        vec![Role::BuyerNegotiator],
        vec![Scope::ListingRead],
    );
    let err = authorize_open_negotiation(&claims, "buyer-1").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

// -----------------------------------------------------------------------
// 5. authorize_request_contact_reveal — participant check
// -----------------------------------------------------------------------

#[test]
fn request_contact_reveal_allows_buyer_participant() {
    assert!(authorize_request_contact_reveal(&buyer_claims(), "seller-1", "buyer-1").is_ok());
}

#[test]
fn request_contact_reveal_allows_seller_participant() {
    assert!(authorize_request_contact_reveal(&seller_claims(), "seller-1", "buyer-1").is_ok());
}

#[test]
fn request_contact_reveal_rejects_non_participant() {
    let err =
        authorize_request_contact_reveal(&buyer_claims(), "seller-999", "buyer-999").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn request_contact_reveal_allows_admin() {
    assert!(authorize_request_contact_reveal(&admin_claims(), "any-seller", "any-buyer").is_ok());
}

#[test]
fn request_contact_reveal_rejects_missing_role() {
    let claims = claims_with_role_and_scopes(
        "no-role",
        vec![Role::SellerListingWriter],
        vec![Scope::NegotiationRevealRequest],
    );
    let err = authorize_request_contact_reveal(&claims, "seller-1", "no-role").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingRole);
}

// -----------------------------------------------------------------------
// 6. authorize_approve_contact_reveal — seller/owner check
// -----------------------------------------------------------------------

#[test]
fn approve_contact_reveal_allows_owner_seller() {
    assert!(authorize_approve_contact_reveal(&seller_claims(), "seller-1").is_ok());
}

#[test]
fn approve_contact_reveal_rejects_wrong_seller() {
    let err = authorize_approve_contact_reveal(&seller_claims(), "seller-999").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn approve_contact_reveal_allows_admin_any_owner() {
    assert!(authorize_approve_contact_reveal(&admin_claims(), "any-seller").is_ok());
}

#[test]
fn approve_contact_reveal_rejects_buyer_without_scope() {
    let err = authorize_approve_contact_reveal(&buyer_claims(), "seller-1").unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

// -----------------------------------------------------------------------
// 7. Generic authorize function — scope, role, ownership, admin bypass
// -----------------------------------------------------------------------

#[test]
fn authorize_rejects_missing_scope() {
    let claims = claims_with_role_and_scopes(
        "seller-1",
        vec![Role::SellerListingWriter],
        vec![], // no scopes at all
    );
    let err = authorize(
        &claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "seller-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

#[test]
fn authorize_rejects_missing_role_for_non_admin() {
    let claims = claims_with_role_and_scopes(
        "reader",
        vec![Role::BuyerSearcher],
        vec![Scope::ListingCreate],
    );
    let err = authorize(
        &claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "reader".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingRole);
}

#[test]
fn authorize_seller_owned_accepts_exact_match() {
    assert!(authorize(
        &seller_claims(),
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "seller-1".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_seller_owned_rejects_mismatch() {
    let err = authorize(
        &seller_claims(),
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "seller-999".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn authorize_admin_bypasses_ownership_check() {
    assert!(authorize(
        &admin_claims(),
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "any-seller".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_admin_bypasses_role_check() {
    // Admin has no seller_listing_writer role but should still pass
    let claims = Claims {
        sub: "admin-1".into(),
        roles: vec![Role::Admin],
        scopes: vec![Scope::ListingCreate],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(authorize(
        &claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "any-seller".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_buyer_owned_accepts_exact_match() {
    assert!(authorize(
        &buyer_claims(),
        Action::OpenNegotiation,
        OwnershipContext::BuyerOwned {
            buyer_agent_id: "buyer-1".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_buyer_owned_rejects_mismatch() {
    let err = authorize(
        &buyer_claims(),
        Action::OpenNegotiation,
        OwnershipContext::BuyerOwned {
            buyer_agent_id: "buyer-999".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn authorize_negotiation_participant_accepts_seller() {
    assert!(authorize(
        &seller_claims(),
        Action::RequestContactReveal,
        OwnershipContext::NegotiationParticipant {
            seller_account_id: "seller-1".into(),
            buyer_agent_id: "buyer-1".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_negotiation_participant_accepts_buyer() {
    assert!(authorize(
        &buyer_claims(),
        Action::RequestContactReveal,
        OwnershipContext::NegotiationParticipant {
            seller_account_id: "seller-1".into(),
            buyer_agent_id: "buyer-1".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_negotiation_participant_rejects_non_participant() {
    let err = authorize(
        &buyer_claims(),
        Action::RequestContactReveal,
        OwnershipContext::NegotiationParticipant {
            seller_account_id: "seller-1".into(),
            buyer_agent_id: "buyer-999".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::OwnershipMismatch);
}

#[test]
fn authorize_admin_bypasses_negotiation_participant() {
    assert!(authorize(
        &admin_claims(),
        Action::RequestContactReveal,
        OwnershipContext::NegotiationParticipant {
            seller_account_id: "any-seller".into(),
            buyer_agent_id: "any-buyer".into(),
        },
    )
    .is_ok());
}

#[test]
fn authorize_support_reader_can_read_listing() {
    assert!(authorize(
        &support_claims(),
        Action::GetListing,
        OwnershipContext::None,
    )
    .is_ok());
}

#[test]
fn authorize_support_reader_cannot_create_listing() {
    let err = authorize(
        &support_claims(),
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "seller-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.kind, AuthzErrorKind::MissingScope);
}

// -----------------------------------------------------------------------
// 8. Edge case: expired tokens
// -----------------------------------------------------------------------

#[test]
fn authorize_expired_token_still_passes_scope_role_checks() {
    // The auth module does not check expiration (that's the caller's job)
    let mut claims = seller_claims();
    claims.exp = Some(1); // expired in 1970
                          // It should still pass authz — expiry is checked by JWT validation layer
    assert!(authorize_create_listing(&claims, "seller-1").is_ok());
}

// -----------------------------------------------------------------------
// 9. AuthzError display
// -----------------------------------------------------------------------

#[test]
fn authz_error_display_includes_kind_and_message() {
    let err = AuthzError::new(AuthzErrorKind::MissingRole, "you are not authorized");
    let msg = format!("{err}");
    assert!(msg.contains("MissingRole"));
    assert!(msg.contains("not authorized"));
}

#[test]
fn authz_error_implements_error_trait() {
    use std::error::Error;
    let err = AuthzError::new(AuthzErrorKind::MissingScope, "missing scope");
    let source = err.source();
    assert!(source.is_none()); // no inner source
}
