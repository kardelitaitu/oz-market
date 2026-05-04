use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SellerListingWriter,
    SellerNegotiator,
    SellerContactRevealApprover,
    BuyerSearcher,
    BuyerNegotiator,
    Admin,
    SupportReviewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    #[serde(rename = "listing:create")]
    ListingCreate,
    #[serde(rename = "listing:read")]
    ListingRead,
    #[serde(rename = "listing:search")]
    ListingSearch,
    #[serde(rename = "negotiation:create")]
    NegotiationCreate,
    #[serde(rename = "negotiation:read")]
    NegotiationRead,
    #[serde(rename = "negotiation:offer:submit")]
    NegotiationOfferSubmit,
    #[serde(rename = "negotiation:reveal:request")]
    NegotiationRevealRequest,
    #[serde(rename = "reveal:approve")]
    RevealApprove,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Claims {
    pub subject: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buyer_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl Claims {
    pub fn has_scope(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }
}
