use serde::{Deserialize, Serialize};

pub type ResourceId = String;
pub type CurrencyCode = String;
pub type CountryCode = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Laptop,
    Phone,
    Tablet,
    Desktop,
    Monitor,
    Accessory,
    Camera,
    Audio,
    Gaming,
    Appliance,
    Furniture,
    VehiclePart,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    New,
    Used,
    Refurbished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    Draft,
    Active,
    Reserved,
    Sold,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    Relevance,
    Newest,
    PriceAsc,
    PriceDesc,
    RatingHighest,  // Phase B: Sort by seller rating descending
    RatingLowest,   // Phase B: Sort by seller rating ascending
}

impl Default for SearchSort {
    fn default() -> Self {
        Self::Relevance
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Price {
    pub currency: CurrencyCode,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingLocation {
    pub country_code: CountryCode,
    pub country_name: String,
    pub city: String,
    // Phase D: Geolocation (optional, seller can opt out)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geolocation_opt_out: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShippingInfo {
    pub local_pickup: bool,
    pub shipping_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<Price>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_regions: Option<Vec<CountryCode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingPayload {
    pub schema_version: String,
    pub owner_id: String,
    pub category: Category,
    pub product_name: String,
    pub condition: Condition,
    pub price: Price,
    pub location: ListingLocation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub picture_urls: Vec<String>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<serde_json::Value>,
    // NEW: Marketplace fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(default)]
    pub quantity: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_info: Option<ShippingInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_details: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateListingRequest {
    pub idempotency_key: String,
    pub listing: ListingPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingSummary {
    pub listing_id: ResourceId,
    pub status: ListingStatus,
    pub version: u64,
    pub listing: ListingPayload,
    // NEW: Seller summary (read-only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_rating: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_verified: Option<bool>,
}

pub type CreateListingResponse = ListingSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchPriceFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<CurrencyCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchLocationFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_code: Option<CountryCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<SearchPriceFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SearchLocationFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ListingStatus>,
    // Phase A: Faceted search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_seller_rating: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_sellers_only: Option<bool>,
    #[serde(default)]
    pub sort_by: SearchSort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    // Phase D: Geolocation search
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near_me: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_longitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_km: Option<f64>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            condition: None,
            price: None,
            location: None,
            status: None,
            min_seller_rating: None,
            verified_sellers_only: None,
            sort_by: SearchSort::Relevance,
            limit: None,
            cursor: None,
            // Phase D: Geolocation
            near_me: None,
            user_latitude: None,
            user_longitude: None,
            radius_km: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResponse {
    pub items: Vec<ListingSummary>,
    pub applied_sort_by: SearchSort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}
