use crate::listing::{CurrencyCode, ResourceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NegotiationStatus {
    Open,
    Countered,
    NearClose,
    Reserved,
    ContactRequested,
    ContactRevealed,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContactRevealStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct OpenNegotiationRequest {
    pub listing_id: ResourceId,
    pub buyer_agent_id: String,
    pub offer_currency: CurrencyCode,
    pub offer_amount: f64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct SubmitOfferRequest {
    pub offer_currency: CurrencyCode,
    pub offer_amount: f64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct AcceptNegotiationRequest {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct RejectNegotiationRequest {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct RequestContactRevealRequest {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NegotiationHistoryEntryType {
    Offer,
    Accept,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct NegotiationHistoryEntry {
    pub entry_id: String,
    pub entry_type: NegotiationHistoryEntryType,
    pub offer_currency: CurrencyCode,
    pub offer_amount: f64,
    pub actor_subject: String,
    pub actor_role: String,
    pub idempotency_key: String,
    pub resulting_status: NegotiationStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct NegotiationResponse {
    pub negotiation_id: ResourceId,
    pub listing_id: ResourceId,
    pub buyer_agent_id: String,
    pub status: NegotiationStatus,
    pub offer_currency: CurrencyCode,
    pub latest_offer_amount: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservation_lease_id: Option<ResourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_offer_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reveal_id: Option<ResourceId>,
    #[serde(default)]
    pub offer_history: Vec<NegotiationHistoryEntry>,
    pub version: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct ContactRevealResponse {
    pub reveal_id: ResourceId,
    pub negotiation_id: ResourceId,
    pub reveal_status: ContactRevealStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revealed_phone_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    pub updated_at: String,
}
