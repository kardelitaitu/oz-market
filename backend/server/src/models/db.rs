use marketplace_api_contract::{
    Category, Condition, ContactRevealStatus, CurrencyCode, ListingPayload, ListingStatus,
    NegotiationStatus, ResourceId,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ListingRow {
    pub listing_id: ResourceId,
    pub owner_id: String,
    pub schema_version: String,
    pub category: Category,
    pub product_name: String,
    pub item_condition: Condition,
    pub price_currency: CurrencyCode,
    pub price_amount: f64,
    pub country_code: String,
    pub country_name: String,
    pub city: String,
    pub picture_urls: Vec<String>,
    pub description: String,
    pub attributes: Option<Value>,
    pub status: ListingStatus,
    pub version: i64,
    pub create_idempotency_key: String,
    pub search_text: String,
    pub created_at: String,
    pub updated_at: String,
    // NEW: Marketplace fields
    pub sku: Option<String>,
    pub quantity: i32,
    pub shipping_info: Option<Value>,
    pub condition_details: Option<String>,
    pub seller_notes: Option<String>,
    // Phase D: Geolocation
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub geolocation_opt_out: Option<bool>,
    // NEW: Phase 2 - Marketplace expansion
    pub listing_type: String, // "product", "service", or "property"
}

impl ListingRow {
    pub fn into_payload(self) -> ListingPayload {
        // Convert listing_type string to enum
        let listing_type_enum = match self.listing_type.as_str() {
            "service" => marketplace_api_contract::ListingType::Service,
            "property" => marketplace_api_contract::ListingType::Property,
            _ => marketplace_api_contract::ListingType::Product,
        };

        ListingPayload {
            schema_version: self.schema_version,
            owner_id: self.owner_id,
            listing_type: listing_type_enum,
            category: if listing_type_enum == marketplace_api_contract::ListingType::Product {
                Some(self.category)
            } else {
                None
            },
            title: self.product_name, // Maps to title in api-contract
            condition: if listing_type_enum == marketplace_api_contract::ListingType::Product {
                Some(self.item_condition)
            } else {
                None
            },
            price: marketplace_api_contract::Price {
                currency: self.price_currency,
                amount: self.price_amount,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: self.country_code,
                country_name: self.country_name,
                city: self.city,
                latitude: self.latitude,
                longitude: self.longitude,
                geolocation_opt_out: self.geolocation_opt_out,
            },
            picture_urls: self.picture_urls,
            description: self.description,
            attributes: self.attributes,
            // NEW: Marketplace fields
            sku: self.sku,
            quantity: if self.quantity == 1 {
                None
            } else {
                Some(self.quantity as u32)
            },
            shipping_info: self
                .shipping_info
                .and_then(|v| serde_json::from_value(v).ok()),
            condition_details: self.condition_details,
            seller_notes: self.seller_notes,
            // TODO: Phase 4 - Populate from separate tables
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegotiationRow {
    pub negotiation_id: ResourceId,
    pub listing_id: ResourceId,
    pub buyer_agent_id: String,
    pub status: NegotiationStatus,
    pub offer_currency: CurrencyCode,
    pub latest_offer_amount: f64,
    pub reservation_lease_id: Option<ResourceId>,
    pub final_offer_amount: Option<f64>,
    pub version: i64,
    pub open_idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservationLeaseRow {
    pub lease_id: ResourceId,
    pub negotiation_id: ResourceId,
    pub listing_id: ResourceId,
    pub reserved_by: String,
    pub status: String,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactRevealRow {
    pub reveal_id: ResourceId,
    pub negotiation_id: ResourceId,
    pub listing_id: ResourceId,
    pub buyer_agent_id: String,
    pub request_idempotency_key: String,
    pub reveal_status: ContactRevealStatus,
    pub revealed_phone_reference: Option<String>,
    pub expires_at: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditEventRow {
    pub event_id: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub actor_subject: String,
    pub actor_role: String,
    pub scopes: Vec<String>,
    pub request_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEventRow {
    pub event_id: i64,
    pub topic: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: Value,
    pub available_at: String,
    pub published_at: Option<String>,
    pub attempt_count: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SellerAccountRow {
    pub seller_account_id: ResourceId,
    pub owner_id: String,
    pub display_name: Option<String>,
    pub trust_level: String,
    pub seller_rating: Option<f64>,
    pub quota_override: Option<i32>,
    pub listings_created: i32,
    pub status: String,
    pub hardware_fingerprint: Option<String>,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReviewRow {
    pub review_id: ResourceId,
    pub listing_id: String,
    pub seller_account_id: String,
    pub reviewer_id: String,
    pub rating: i32,
    pub title: String,
    pub body: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCredentialRow {
    pub credential_id: ResourceId,
    pub seller_account_id: ResourceId,
    pub subject: String,
    pub role: String,
    pub scopes: Vec<String>,
    pub revoked_at: Option<String>,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyKeyStatus {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdempotencyKeyRow {
    pub idempotency_key: String,
    pub actor_subject: String,
    pub operation: String,
    pub request_fingerprint: String,
    pub status: IdempotencyKeyStatus,
    pub response_payload: Option<Value>,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
}
