use oz_market_auth_core::{Action, Claims, OwnershipContext, Role, Scope};

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
    use oz_market_auth_core::action_to_scopes;

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
    let result = oz_market_auth_core::extract_token(header);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9"
    );
}

#[test]
fn extract_token_invalid_format() {
    let header = "Token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiJ9";
    let result = oz_market_auth_core::extract_token(header);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        oz_market_auth_core::AuthError::MissingAuthHeader
    ));
}

#[test]
fn extract_token_empty() {
    let header = "Bearer";
    let result = oz_market_auth_core::extract_token(header);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        oz_market_auth_core::AuthError::MissingAuthHeader
    ));
}

// ---------------------------------------------------------------------
// Additional edge-case coverage added during the spec 0006-0017 audit.
// ---------------------------------------------------------------------

#[test]
fn authorize_allows_when_ownership_is_none_and_scope_present() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![Scope::ListingCreate],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(
        oz_market_auth_core::authorize(&claims, Action::CreateListing, OwnershipContext::None)
            .is_ok()
    );
}

#[test]
fn authorize_allows_matching_seller_owner() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![Scope::ListingCreate],
        seller_account_id: Some("seller_42".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(oz_market_auth_core::authorize(
        &claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: "seller_42".to_string(),
        }
    )
    .is_ok());
}

#[test]
fn authorize_rejects_seller_owned_when_claims_have_no_seller_id() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![Scope::ListingCreate],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(matches!(
        oz_market_auth_core::authorize(
            &claims,
            Action::CreateListing,
            OwnershipContext::SellerOwned {
                seller_account_id: "seller_42".to_string(),
            }
        ),
        Err(oz_market_auth_core::AuthError::OwnershipFailed)
    ));
}

#[test]
fn authorize_rejects_negotiation_participant_when_neither_matches() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![Scope::NegotiationCreate],
        seller_account_id: Some("seller_A".to_string()),
        buyer_agent_id: Some("buyer_A".to_string()),
        hardware_id: None,
        exp: None,
    };
    assert!(matches!(
        oz_market_auth_core::authorize(
            &claims,
            Action::OpenNegotiation,
            OwnershipContext::NegotiationParticipant {
                seller_account_id: "seller_B".to_string(),
                buyer_agent_id: "buyer_B".to_string(),
            }
        ),
        Err(oz_market_auth_core::AuthError::OwnershipFailed)
    ));
}

#[test]
fn authorize_rejects_buyer_owned_when_claims_have_no_buyer_id() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![Scope::NegotiationCreate],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(matches!(
        oz_market_auth_core::authorize(
            &claims,
            Action::OpenNegotiation,
            OwnershipContext::BuyerOwned {
                buyer_agent_id: "buyer_42".to_string(),
            }
        ),
        Err(oz_market_auth_core::AuthError::OwnershipFailed)
    ));
}

#[test]
fn authorize_with_empty_scopes_rejects_any_action() {
    let claims = Claims {
        sub: "agent_123".to_string(),
        roles: vec![],
        scopes: vec![],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    };
    assert!(matches!(
        oz_market_auth_core::authorize(&claims, Action::SearchListings, OwnershipContext::None),
        Err(oz_market_auth_core::AuthError::InsufficientScope(_))
    ));
}

#[test]
fn scope_serde_roundtrip_snake_case() {
    // Use the Scope's own Serialize/Deserialize: confirm rename_all = "snake_case".
    let scope = Scope::NegotiationOfferSubmit;
    let json = serde_json::to_string(&scope).unwrap();
    assert_eq!(json, "\"negotiation:offer:submit\"");
    let back: Scope = serde_json::from_str(&json).unwrap();
    assert_eq!(back, scope);
}

#[test]
fn role_serde_roundtrip_snake_case() {
    let role = Role::SellerContactRevealApprover;
    let json = serde_json::to_string(&role).unwrap();
    assert_eq!(json, "\"seller_contact_reveal_approver\"");
    let back: Role = serde_json::from_str(&json).unwrap();
    assert_eq!(back, role);
}

#[test]
fn validate_token_rejects_garbage_string() {
    let result = oz_market_auth_core::validate_token("not.a.valid.jwt", b"secret");
    assert!(matches!(
        result,
        Err(oz_market_auth_core::AuthError::InvalidToken(_))
    ));
}

#[test]
fn validate_token_rejects_empty_string() {
    let result = oz_market_auth_core::validate_token("", b"secret");
    assert!(matches!(
        result,
        Err(oz_market_auth_core::AuthError::InvalidToken(_))
    ));
}

#[test]
fn validate_token_rejects_wrong_signature() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let claims = Claims {
        sub: "agent_123".to_string(),
        scopes: vec![Scope::ListingCreate],
        exp: Some(1_900_000_000),
        ..Default::default()
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"secret-A"),
    )
    .unwrap();
    let result = oz_market_auth_core::validate_token(&token, b"secret-B");
    assert!(matches!(
        result,
        Err(oz_market_auth_core::AuthError::InvalidToken(_))
    ));
}

#[test]
fn extract_token_is_case_sensitive_on_scheme() {
    // "bearer xxx" must be rejected (HTTP specifies Bearer as case-insensitive,
    // but this implementation is strict — pin the contract).
    let result = oz_market_auth_core::extract_token("bearer abc.def.ghi");
    assert!(matches!(
        result,
        Err(oz_market_auth_core::AuthError::MissingAuthHeader)
    ));
}

#[test]
fn extract_token_with_token_containing_spaces_rejected() {
    // "Bearer abc def" splits into 4 parts, doesn't match ["Bearer", token].
    let result = oz_market_auth_core::extract_token("Bearer abc def ghi");
    assert!(matches!(
        result,
        Err(oz_market_auth_core::AuthError::MissingAuthHeader)
    ));
}

#[test]
fn default_claims_is_empty() {
    let claims = Claims::default();
    assert_eq!(claims.sub, "");
    assert!(claims.roles.is_empty());
    assert!(claims.scopes.is_empty());
    assert!(claims.seller_account_id.is_none());
    assert!(claims.buyer_agent_id.is_none());
    assert!(claims.hardware_id.is_none());
    assert!(claims.exp.is_none());
    assert!(!claims.is_expired());
}
