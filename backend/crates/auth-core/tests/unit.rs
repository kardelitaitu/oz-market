use marketplace_auth_core::{Action, Claims, OwnershipContext, Role, Scope};

#[test]
fn claims_has_scope() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![Role::SellerListingWriter],
        scopes: vec![
            Scope::ListingCreate,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
        ],
        seller_account_id: Some("seller_123".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };

    assert!(claims.has_scope(Scope::ListingCreate));
    assert!(claims.has_scope(Scope::ListingSearch));
    assert!(!claims.has_scope(Scope::RevealApprove));
}

#[test]
fn claims_has_role() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![Role::SellerListingWriter, Role::BuyerNegotiator],
        scopes: vec![],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };

    assert!(claims.has_role(Role::SellerListingWriter));
    assert!(claims.has_role(Role::BuyerNegotiator));
    assert!(!claims.has_role(Role::Admin));
}

#[test]
fn claims_expired() {
    let expired_claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: Some(1_000_000_000), // expired in 2001
    };

    assert!(expired_claims.is_expired());
}

#[test]
fn claims_not_expired() {
    let valid_claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: Some(1_900_000_000), // expires in 2028
    };

    assert!(!valid_claims.is_expired());
}

#[test]
fn claims_no_expiry() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };

    assert!(!claims.is_expired());
}

#[test]
fn action_to_scopes_mapping() {
    // Test that each action maps to expected scope(s)
    use marketplace_auth_core::action_to_scopes;

    let create_scopes = action_to_scopes(Action::CreateListing);
    assert_eq!(create_scopes, vec![Scope::ListingCreate]);

    let search_scopes = action_to_scopes(Action::SearchListings);
    assert_eq!(search_scopes, vec![Scope::ListingSearch]);

    let negotiate_scopes = action_to_scopes(Action::OpenNegotiation);
    assert_eq!(negotiate_scopes, vec![Scope::NegotiationCreate]);

    let reveal_scopes = action_to_scopes(Action::RequestContactReveal);
    assert_eq!(reveal_scopes, vec![Scope::NegotiationRevealRequest]);
}

#[test]
fn ownership_context_comparison() {
    let seller_owned = OwnershipContext::SellerOwned {
        seller_account_id: "seller_123".to_string(),
    };

    let buyer_owned = OwnershipContext::BuyerOwned {
        buyer_agent_id: "buyer_123".to_string(),
    };

    let negotiation = OwnershipContext::NegotiationParticipant {
        seller_account_id: "seller_123".to_string(),
        buyer_agent_id: "buyer_123".to_string(),
    };

    let none = OwnershipContext::None;

    // Test equality for same context
    assert!(matches!(seller_owned, OwnershipContext::SellerOwned { .. }));
    assert!(matches!(buyer_owned, OwnershipContext::BuyerOwned { .. }));
    assert!(matches!(
        negotiation,
        OwnershipContext::NegotiationParticipant { .. }
    ));
    assert!(matches!(none, OwnershipContext::None));
}

#[test]
fn extract_token_valid_bearer() {
    let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9";
    let result = marketplace_auth_core::extract_token(header);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9"
    );
}

#[test]
fn extract_token_invalid_format() {
    let header = "Token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9";
    let result = marketplace_auth_core::extract_token(header);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        marketplace_auth_core::AuthError::MissingAuthHeader
    ));
}

#[test]
fn extract_token_empty() {
    let header = "Bearer";
    let result = marketplace_auth_core::extract_token(header);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        marketplace_auth_core::AuthError::MissingAuthHeader
    ));
}
