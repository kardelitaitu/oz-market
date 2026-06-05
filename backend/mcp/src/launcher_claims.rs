use oz_market_auth_core::{Claims, Role, Scope};

pub fn dev_launcher_claims() -> Claims {
    Claims {
        sub: "mcp-agent-dev".to_string(),
        roles: vec![
            Role::SellerListingWriter,
            Role::SellerNegotiator,
            Role::SellerContactRevealApprover,
            Role::BuyerSearcher,
            Role::BuyerNegotiator,
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
        seller_account_id: Some("seller-1".to_string()),
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: None,
    }
}

pub fn dev_launcher_claims_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(&dev_launcher_claims())
}
