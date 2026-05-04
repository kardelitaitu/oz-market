use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
    pub attributes: Option<BTreeMap<String, Value>>,
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
    #[serde(default)]
    pub sort_by: SearchSort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
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
            sort_by: SearchSort::Relevance,
            limit: None,
            cursor: None,
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
