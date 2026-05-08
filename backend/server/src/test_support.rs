use marketplace_auth_core::{Claims, Role, Scope};

pub fn seller_claims() -> Claims {
    Claims {
        sub: "sub-1".to_string(),
        roles: vec![
            Role::SellerListingWriter,
            Role::BuyerNegotiator,
            Role::SellerContactRevealApprover,
        ],
        scopes: vec![
            Scope::ListingCreate,
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationRevealRequest,
            Scope::RevealApprove,
        ],
        seller_account_id: Some("seller-1".to_string()),
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: None,
    }
}

pub fn admin_claims() -> Claims {
    Claims {
        sub: "admin-1".to_string(),
        roles: vec![Role::Admin],
        scopes: vec![Scope::ListingRead, Scope::NegotiationRead],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

pub fn support_claims() -> Claims {
    Claims {
        sub: "support-1".to_string(),
        roles: vec![Role::SupportReviewer],
        scopes: vec![Scope::ListingRead, Scope::NegotiationRead],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}
