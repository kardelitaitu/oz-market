use crate::models::db::ListingRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};
use marketplace_api_contract::{
    CreateListingRequest, CreateListingResponse, ListingStatus, ListingSummary, SearchRequest,
    SearchResponse,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sqlx::{
    postgres::{PgPool, PgRow, Postgres},
    types::Json,
    QueryBuilder, Row,
};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
}

#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn insert_listing(
        &self,
        request: &CreateListingRequest,
    ) -> Result<CreateListingResponse, RepositoryError>;

    async fn get_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, RepositoryError>;

    async fn search_listings(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, RepositoryError>;

    async fn update_listing_status(
        &self,
        listing_id: &str,
        status: ListingStatus,
    ) -> Result<Option<ListingSummary>, RepositoryError>;
}

pub fn conflict(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Conflict, message)
}

pub struct InMemoryListingRepository {
    listings: RwLock<HashMap<String, ListingSummary>>,
    next_listing_id: AtomicU64,
}

impl InMemoryListingRepository {
    pub fn new() -> Self {
        Self {
            listings: RwLock::new(HashMap::new()),
            next_listing_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let next = self.next_listing_id.fetch_add(1, Ordering::SeqCst);
        format!("lst_{next:06}")
    }

    fn summary_to_row(summary: &ListingSummary) -> ListingRow {
        ListingRow {
            listing_id: summary.listing_id.clone(),
            owner_id: summary.listing.owner_id.clone(),
            schema_version: summary.listing.schema_version.clone(),
            category: summary
                .listing
                .category
                .unwrap_or(marketplace_api_contract::Category::Laptop),
            product_name: summary.listing.title.clone(), // Maps title -> product_name for DB
            item_condition: summary
                .listing
                .condition
                .unwrap_or(marketplace_api_contract::Condition::Used),
            price_currency: summary.listing.price.currency.clone(),
            price_amount: summary.listing.price.amount,
            country_code: summary.listing.location.country_code.clone(),
            country_name: summary.listing.location.country_name.clone(),
            city: summary.listing.location.city.clone(),
            picture_urls: summary.listing.picture_urls.clone(),
            description: summary.listing.description.clone(),
            attributes: summary.listing.attributes.clone(),
            // NEW: Marketplace fields
            sku: summary.listing.sku.clone(),
            quantity: summary.listing.quantity.map(|q| q as i32),
            shipping_info: summary
                .listing
                .shipping_info
                .as_ref()
                .map(|si| serde_json::to_value(si).unwrap_or(serde_json::Value::Null)),
            condition_details: summary.listing.condition_details.clone(),
            seller_notes: summary.listing.seller_notes.clone(),
            status: summary.status,
            version: summary.version as i64,
            create_idempotency_key: String::new(),
            search_text: crate::services::search::listing_index_text(&summary.listing),
            created_at: String::new(),
            updated_at: String::new(),
            // Phase D: Geolocation fields
            latitude: summary.listing.location.latitude,
            longitude: summary.listing.location.longitude,
            geolocation_opt_out: summary.listing.location.geolocation_opt_out,
            // NEW: Phase 2
            listing_type: match summary.listing.listing_type {
                marketplace_api_contract::ListingType::Service => "service",
                marketplace_api_contract::ListingType::Property => "property",
                _ => "product",
            }
            .to_string(),
        }
    }
}

impl Default for InMemoryListingRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ListingRepository for InMemoryListingRepository {
    async fn insert_listing(
        &self,
        request: &CreateListingRequest,
    ) -> Result<CreateListingResponse, RepositoryError> {
        let mut guard = self.listings.write().expect("listing write lock");
        if guard.values().any(|listing| {
            listing.listing.owner_id == request.listing.owner_id
                && listing.listing.title == request.listing.title
        }) {
            return Err(conflict("duplicate listing fingerprint"));
        }

        let summary = ListingSummary {
            listing_id: self.next_id(),
            status: ListingStatus::Active,
            version: 1,
            listing: request.listing.clone(),
            // Seller fields (read-only, None for in-memory)
            seller_name: None,
            seller_rating: None,
            seller_verified: None,
        };
        let _ = Self::summary_to_row(&summary);
        guard.insert(summary.listing_id.clone(), summary.clone());
        Ok(summary)
    }

    async fn get_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, RepositoryError> {
        let guard = self.listings.read().expect("listing read lock");
        Ok(guard.get(listing_id).cloned())
    }

    async fn search_listings(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, RepositoryError> {
        let guard = self.listings.read().expect("listing read lock");
        let mut items: Vec<ListingSummary> = guard
            .values()
            .filter(|listing| matches_filters(listing, request))
            .cloned()
            .collect();

        let query_terms = request
            .query
            .as_deref()
            .map(crate::services::search::normalize_search_terms)
            .unwrap_or_default();

        items.sort_by(|a, b| {
            crate::services::search::compare_search_items(a, b, &query_terms, request.sort_by)
        });

        if let Some(cursor) = request.cursor.as_deref() {
            if let Some(index) = items.iter().position(|item| item.listing_id == cursor) {
                items = items.into_iter().skip(index + 1).collect();
            }
        }

        let limit = request.limit.unwrap_or(20).min(50) as usize;
        let next_cursor = if items.len() > limit {
            items.get(limit - 1).map(|item| item.listing_id.clone())
        } else {
            None
        };
        items.truncate(limit);

        Ok(SearchResponse {
            items,
            applied_sort_by: request.sort_by,
            next_cursor,
        })
    }

    async fn update_listing_status(
        &self,
        listing_id: &str,
        status: ListingStatus,
    ) -> Result<Option<ListingSummary>, RepositoryError> {
        let mut guard = self.listings.write().expect("listing write lock");
        if let Some(listing) = guard.get_mut(listing_id) {
            listing.status = status;
            listing.version += 1;
            Ok(Some(listing.clone()))
        } else {
            Ok(None)
        }
    }
}

fn db_enum_value<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("enum serialization")
        .trim_matches('"')
        .to_string()
}

fn parse_db_enum<T: DeserializeOwned>(value: &str) -> Result<T, RepositoryError> {
    serde_json::from_value(Value::String(value.to_string()))
        .map_err(|error| storage(error.to_string()))
}

fn matches_filters(listing: &ListingSummary, request: &SearchRequest) -> bool {
    if let Some(category) = request.category {
        if listing.listing.category != Some(category) {
            return false;
        }
    }

    if let Some(condition) = request.condition {
        if listing.listing.condition != Some(condition) {
            return false;
        }
    }

    if let Some(status) = request.status {
        if listing.status != status {
            return false;
        }
    }

    if let Some(listing_type) = request.listing_type {
        if listing.listing.listing_type != listing_type {
            return false;
        }
    }

    if let Some(price_filter) = &request.price {
        if let Some(currency) = &price_filter.currency {
            if listing.listing.price.currency != *currency {
                return false;
            }
        }

        if let Some(min_amount) = price_filter.min_amount {
            if listing.listing.price.amount < min_amount {
                return false;
            }
        }

        if let Some(max_amount) = price_filter.max_amount {
            if listing.listing.price.amount > max_amount {
                return false;
            }
        }
    }

    if let Some(location_filter) = &request.location {
        if let Some(country_code) = &location_filter.country_code {
            if listing.listing.location.country_code != *country_code {
                return false;
            }
        }

        if let Some(city) = &location_filter.city {
            if !listing
                .listing
                .location
                .city
                .to_lowercase()
                .contains(&city.to_lowercase())
            {
                return false;
            }
        }
    }

    // Service filters
    if let Some(service_type) = request.service_type {
        if listing.listing.service_type != Some(service_type) {
            return false;
        }
    }

    // Property filters
    if let Some(prop_transaction_type) = request.property_transaction_type {
        if listing.listing.property_transaction_type != Some(prop_transaction_type) {
            return false;
        }
    }

    if let Some(prop_sub_type) = request.property_sub_type {
        if listing.listing.property_sub_type != Some(prop_sub_type) {
            return false;
        }
    }

    if let Some(min_area) = request.min_area_sqm {
        if let Some(area) = listing.listing.area_sqm {
            if area < min_area {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(max_area) = request.max_area_sqm {
        if let Some(area) = listing.listing.area_sqm {
            if area > max_area {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(min_bed) = request.min_bedrooms {
        if let Some(beds) = listing.listing.bedrooms {
            if beds < min_bed {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(min_bath) = request.min_bathrooms {
        if let Some(baths) = listing.listing.bathrooms {
            if baths < min_bath {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(ref owner_id) = request.owner_id {
        if listing.listing.owner_id != *owner_id {
            return false;
        }
    }

    // Seller filters
    if let Some(min_rating) = request.min_seller_rating {
        if let Some(rating) = listing.seller_rating {
            if rating < min_rating {
                return false;
            }
        } else {
            return false;
        }
    }

    if request.verified_sellers_only.unwrap_or(false) && listing.seller_verified != Some(true) {
        return false;
    }

    true
}

fn row_to_summary(row: PgRow) -> Result<ListingSummary, RepositoryError> {
    let listing_id = row
        .try_get::<String, _>("listing_id")
        .map_err(|error| storage(error.to_string()))?;
    let status = parse_db_enum(
        &row.try_get::<String, _>("status")
            .map_err(|error| storage(error.to_string()))?,
    )?;
    let schema_version = row
        .try_get::<String, _>("schema_version")
        .map_err(|error| storage(error.to_string()))?;
    let category = parse_db_enum(
        &row.try_get::<String, _>("category")
            .map_err(|error| storage(error.to_string()))?,
    )?;
    let product_name = row
        .try_get::<String, _>("product_name")
        .map_err(|error| storage(error.to_string()))?;
    let condition = parse_db_enum(
        &row.try_get::<String, _>("condition")
            .map_err(|error| storage(error.to_string()))?,
    )?;
    let price_currency = row
        .try_get::<String, _>("price_currency")
        .map_err(|error| storage(error.to_string()))?;
    let price_amount = row
        .try_get::<String, _>("price_amount")
        .map_err(|error| storage(error.to_string()))?
        .parse::<f64>()
        .map_err(|error| storage(error.to_string()))?;
    let country_code = row
        .try_get::<String, _>("country_code")
        .map_err(|error| storage(error.to_string()))?;
    let country_name = row
        .try_get::<String, _>("country_name")
        .map_err(|error| storage(error.to_string()))?;
    let city = row
        .try_get::<String, _>("city")
        .map_err(|error| storage(error.to_string()))?;
    let picture_urls = row
        .try_get::<Json<Vec<String>>, _>("picture_urls")
        .map_err(|error| storage(error.to_string()))?
        .0;
    let description = row
        .try_get::<String, _>("description")
        .map_err(|error| storage(error.to_string()))?;
    let attributes = row
        .try_get::<Option<Json<Value>>, _>("attributes")
        .map_err(|error| storage(error.to_string()))?
        .map(|value| value.0);
    let version = row
        .try_get::<i64, _>("version")
        .map_err(|error| storage(error.to_string()))? as u64;

    // NEW: Extract marketplace fields from row
    let sku = row
        .try_get::<Option<String>, _>("sku")
        .map_err(|error| storage(error.to_string()))?;
    let quantity = row
        .try_get::<Option<i32>, _>("quantity")
        .map_err(|error| storage(error.to_string()))?
        .unwrap_or(0);
    let shipping_info = row
        .try_get::<Option<serde_json::Value>, _>("shipping_info")
        .map_err(|error| storage(error.to_string()))?;
    let condition_details = row
        .try_get::<Option<String>, _>("condition_details")
        .map_err(|error| storage(error.to_string()))?;
    let seller_notes = row
        .try_get::<Option<String>, _>("seller_notes")
        .map_err(|error| storage(error.to_string()))?;

    // NEW: Extract listing_type (default to "product" for old rows)
    let listing_type_str: String = row
        .try_get("listing_type")
        .ok()
        .unwrap_or_else(|| "product".to_string());
    let listing_type: marketplace_api_contract::ListingType = if listing_type_str == "service" {
        marketplace_api_contract::ListingType::Service
    } else if listing_type_str == "property" {
        marketplace_api_contract::ListingType::Property
    } else {
        marketplace_api_contract::ListingType::Product
    };

    // Seller fields (optional, from JOIN with seller_accounts)
    // Use ok() to handle missing columns gracefully
    let display_name: Option<String> = row.try_get("display_name").ok();
    let seller_rating: Option<f64> = row.try_get("seller_rating").ok();
    let verified_at: Option<String> = row.try_get("verified_at").ok();

    let seller_verified = verified_at.is_some();
    let seller_name = display_name;

    // Phase D: Extract geolocation fields (optional)
    let latitude: Option<f64> = row.try_get("latitude").ok();
    let longitude: Option<f64> = row.try_get("longitude").ok();
    let geolocation_opt_out: Option<bool> = row.try_get("geolocation_opt_out").ok();

    // NEW: Phase 4 - Extract fields from LEFT JOINed tables
    // Service fields (from service_listings sl)
    let sl_service_type: Option<String> = row.try_get("service_type").ok();
    let sl_hourly_rate: Option<f64> = row.try_get("hourly_rate").ok();
    let sl_project_rate: Option<f64> = row.try_get("project_rate").ok();
    let sl_qualifications: Option<serde_json::Value> = row.try_get("qualifications").ok();
    let sl_service_radius_km: Option<i32> = row.try_get("service_radius_km").ok();

    // Property fields (from property_listings pl)
    let pl_property_transaction_type: Option<String> =
        row.try_get("property_transaction_type").ok();
    let pl_property_sub_type: Option<String> = row.try_get("property_sub_type").ok();
    let pl_area_sqm: Option<f64> = row.try_get("area_sqm").ok();
    let pl_bedrooms: Option<i32> = row.try_get("bedrooms").ok();
    let pl_bathrooms: Option<i32> = row.try_get("bathrooms").ok();
    let pl_year_built: Option<i32> = row.try_get("year_built").ok();
    let pl_lot_size_sqm: Option<f64> = row.try_get("lot_size_sqm").ok();
    let pl_zoning: Option<String> = row.try_get("zoning").ok();

    Ok(ListingSummary {
        listing_id,
        status,
        version,
        listing: marketplace_api_contract::ListingPayload {
            schema_version,
            owner_id: row
                .try_get::<String, _>("owner_id")
                .map_err(|error| storage(error.to_string()))?,
            listing_type,
            category: if listing_type == marketplace_api_contract::ListingType::Product {
                Some(category)
            } else {
                None
            },
            title: product_name,
            condition: if listing_type == marketplace_api_contract::ListingType::Product {
                Some(condition)
            } else {
                None
            },
            price: marketplace_api_contract::Price {
                currency: price_currency,
                amount: price_amount,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code,
                country_name,
                city,
                // Phase D: Geolocation (optional)
                latitude,
                longitude,
                geolocation_opt_out,
            },
            picture_urls,
            description,
            attributes,
            // NEW: Marketplace fields
            sku,
            quantity: if quantity == 1 {
                None
            } else {
                Some(quantity as u32)
            },
            shipping_info: shipping_info.and_then(|v| serde_json::from_value(v).ok()),
            condition_details,
            seller_notes,
            // NEW: Phase 4 - Service fields (populated from LEFT JOIN)
            service_type: sl_service_type.and_then(|s| {
                if s == "local" {
                    Some(marketplace_api_contract::ServiceType::Local)
                } else if s == "online" {
                    Some(marketplace_api_contract::ServiceType::Online)
                } else {
                    None
                }
            }),
            hourly_rate: sl_hourly_rate,
            project_rate: sl_project_rate,
            qualifications: sl_qualifications.and_then(|v| serde_json::from_value(v).ok()),
            service_radius_km: sl_service_radius_km,
            // NEW: Phase 4 - Property fields (populated from LEFT JOIN)
            property_transaction_type: pl_property_transaction_type.and_then(|s| {
                if s == "rent" {
                    Some(marketplace_api_contract::PropertyTransactionType::Rent)
                } else if s == "sale" {
                    Some(marketplace_api_contract::PropertyTransactionType::Sale)
                } else {
                    None
                }
            }),
            property_sub_type: pl_property_sub_type.and_then(|s| {
                if s == "building" {
                    Some(marketplace_api_contract::PropertySubType::Building)
                } else if s == "house" {
                    Some(marketplace_api_contract::PropertySubType::House)
                } else if s == "apartment" {
                    Some(marketplace_api_contract::PropertySubType::Apartment)
                } else if s == "land" {
                    Some(marketplace_api_contract::PropertySubType::Land)
                } else {
                    None
                }
            }),
            area_sqm: pl_area_sqm,
            bedrooms: pl_bedrooms,
            bathrooms: pl_bathrooms,
            year_built: pl_year_built,
            lot_size_sqm: pl_lot_size_sqm,
            zoning: pl_zoning,
        },
        // Seller fields (read-only)
        seller_name,
        seller_rating,
        seller_verified: Some(seller_verified),
    })
}

pub struct PostgresListingRepository {
    pool: Arc<PgPool>,
}

impl PostgresListingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    #[allow(unused_assignments)]
    async fn fetch_rows(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<ListingSummary>, RepositoryError> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT l.listing_id, l.owner_id, l.schema_version, l.category, l.product_name, l.\"condition\", l.price_currency, l.price_amount::TEXT AS price_amount, l.country_code, l.country_name, l.city, l.picture_urls, l.description, l.attributes, l.status, l.version, l.sku, l.quantity, l.shipping_info, l.condition_details, l.seller_notes, l.listing_type, l.latitude, l.longitude, l.geolocation_opt_out,
                s.display_name, s.seller_rating, s.verified_at,
                sl.service_type, sl.hourly_rate, sl.project_rate, sl.qualifications, sl.service_radius_km,
                pl.property_transaction_type, pl.property_sub_type, pl.area_sqm, pl.bedrooms, pl.bathrooms, pl.year_built, pl.lot_size_sqm, pl.zoning
             FROM listings l
             LEFT JOIN seller_accounts s ON l.owner_id = s.owner_id
             LEFT JOIN service_listings sl ON l.listing_id = sl.listing_id
             LEFT JOIN property_listings pl ON l.listing_id = pl.listing_id",
        );
        let mut where_added = false;

        if let Some(category) = request.category {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("category = ")
                .push_bind(db_enum_value(&category));
        }

        if let Some(condition) = request.condition {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("\"condition\" = ")
                .push_bind(db_enum_value(&condition));
        }

        if let Some(status) = request.status {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("status = ").push_bind(db_enum_value(&status));
        }

        // NEW: Phase 4 - listing_type filter
        if let Some(listing_type) = request.listing_type {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("l.listing_type = ")
                .push_bind(db_enum_value(&listing_type));

            // Note: All JOINs are already in the base query above
        }

        if let Some(price_filter) = &request.price {
            if let Some(currency) = &price_filter.currency {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_currency = ").push_bind(currency);
            }

            if let Some(min_amount) = price_filter.min_amount {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_amount >= ").push_bind(min_amount);
            }

            if let Some(max_amount) = price_filter.max_amount {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_amount <= ").push_bind(max_amount);
            }
        }

        if let Some(location_filter) = &request.location {
            if let Some(country_code) = &location_filter.country_code {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("country_code = ").push_bind(country_code);
            }

            if let Some(city) = &location_filter.city {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder
                    .push("lower(city) = lower(")
                    .push_bind(city.to_lowercase())
                    .push(")");
            }
        }

        // NEW: Service filters
        if let Some(service_type) = request.service_type {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("sl.service_type = ")
                .push_bind(db_enum_value(&service_type));
        }

        // NEW: Property filters
        if let Some(prop_transaction_type) = request.property_transaction_type {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("pl.property_transaction_type = ")
                .push_bind(db_enum_value(&prop_transaction_type));
        }

        if let Some(prop_sub_type) = request.property_sub_type {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder
                .push("pl.property_sub_type = ")
                .push_bind(db_enum_value(&prop_sub_type));
        }

        if let Some(min_area) = request.min_area_sqm {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("pl.area_sqm >= ").push_bind(min_area);
        }

        if let Some(max_area) = request.max_area_sqm {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("pl.area_sqm <= ").push_bind(max_area);
        }

        if let Some(min_bed) = request.min_bedrooms {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("pl.bedrooms >= ").push_bind(min_bed);
        }

        if let Some(min_bath) = request.min_bathrooms {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("pl.bathrooms >= ").push_bind(min_bath);
        }

        // Phase A: Faceted search filters
        if let Some(min_rating) = request.min_seller_rating {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("s.seller_rating >= ").push_bind(min_rating);
        }
        if let Some(true) = request.verified_sellers_only {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("s.verified_at IS NOT NULL");
        }

        if let Some(ref owner_id) = request.owner_id {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("l.owner_id = ").push_bind(owner_id);
        }

        // Phase D: Geolocation search ("near me")
        if let Some(true) = request.near_me {
            if let (Some(user_lat), Some(user_lon)) =
                (request.user_latitude, request.user_longitude)
            {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                // Only include listings that opted in (have coordinates and didn't opt out)
                builder.push("l.latitude IS NOT NULL AND l.longitude IS NOT NULL ");
                builder
                    .push("AND (l.geolocation_opt_out IS NULL OR l.geolocation_opt_out = false) ");

                // Calculate distance using Haversine formula (inline in WHERE)
                let radius_km = 50.0; // Default 50km radius
                builder.push("AND (");
                builder.push("  6371 * acos("); // Earth's radius in km
                builder
                    .push("    sin(radians(")
                    .push_bind(user_lat)
                    .push(") * sin(radians(l.latitude)) + ");
                builder.push("    cos(radians(");
                builder
                    .push_bind(user_lat)
                    .push(")) * cos(radians(l.latitude)) * ");
                builder.push("    cos(radians(l.longitude) - radians(");
                builder.push_bind(user_lon).push("))");
                builder.push("    sin(radians(l.latitude))");
                builder.push("  ) <= ").push_bind(radius_km);
                builder.push(")");

                // Sort by distance for "near me" searches
                builder.push(" ORDER BY ");
                builder.push("  6371 * acos(");
                builder
                    .push("    sin(radians(")
                    .push_bind(user_lat)
                    .push(") * sin(radians(l.latitude)) + ");
                builder.push("    cos(radians(");
                builder
                    .push_bind(user_lat)
                    .push(")) * cos(radians(l.latitude)) * ");
                builder.push("    cos(radians(l.longitude) - radians(");
                builder.push_bind(user_lon).push("))");
                builder.push("    sin(radians(l.latitude))");
                builder.push("  ) ASC, l.listing_id");
            } else {
                // User location not provided - fall back to city/country match
                // (already implemented via existing location filter)
            }
        }

        // Add LIMIT to avoid fetching too many rows
        // Use request limit or default to 50, max 200 to prevent excessive memory use
        let limit = request.limit.unwrap_or(50).min(200);
        builder.push(" LIMIT ");
        builder.push(limit.to_string());

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;
        let rows = builder
            .build()
            .fetch_all(&mut *conn)
            .await
            .map_err(|error| storage(error.to_string()))?;

        let mut items = rows
            .into_iter()
            .map(row_to_summary)
            .collect::<Result<Vec<_>, _>>()?;

        let query_terms = request
            .query
            .as_deref()
            .map(crate::services::search::normalize_search_terms)
            .unwrap_or_default();

        items.sort_by(|a, b| {
            crate::services::search::compare_search_items(a, b, &query_terms, request.sort_by)
        });

        Ok(items)
    }
}

#[async_trait::async_trait]
impl ListingRepository for PostgresListingRepository {
    async fn insert_listing(
        &self,
        request: &CreateListingRequest,
    ) -> Result<CreateListingResponse, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let listing_id = format!("lst_{}", uuid::Uuid::new_v4().simple());
        let listing_summary = ListingSummary {
            listing_id: listing_id.clone(),
            status: ListingStatus::Active,
            version: 1,
            listing: request.listing.clone(),
            seller_name: None,
            seller_rating: None,
            seller_verified: None,
        };

        sqlx::query(
            "INSERT INTO listings (listing_id, owner_id, schema_version, category, product_name, \"condition\", price_currency, price_amount, country_code, country_name, city, picture_urls, description, attributes, status, version, create_idempotency_key, sku, quantity, shipping_info, condition_details, seller_notes, listing_type, latitude, longitude, geolocation_opt_out)
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)",
        )
        .bind(&listing_summary.listing_id)
        .bind(&listing_summary.listing.owner_id)
        .bind(&listing_summary.listing.schema_version)
        .bind(listing_summary.listing.category.map(|c| db_enum_value(&c)))
        .bind(&listing_summary.listing.title)
        .bind(listing_summary.listing.condition.map(|c| db_enum_value(&c)))
        .bind(&listing_summary.listing.price.currency)
        .bind(listing_summary.listing.price.amount)
        .bind(&listing_summary.listing.location.country_code)
        .bind(&listing_summary.listing.location.country_name)
        .bind(&listing_summary.listing.location.city)
        .bind(sqlx::types::Json(&listing_summary.listing.picture_urls))
        .bind(&listing_summary.listing.description)
        .bind(listing_summary.listing.attributes.as_ref().unwrap_or(&serde_json::Value::Null))
        .bind(db_enum_value(&listing_summary.status))
        .bind(listing_summary.version as i64)
        .bind(&request.idempotency_key)
        .bind(&listing_summary.listing.sku)
        .bind(listing_summary.listing.quantity.map(|q| q as i32))
        .bind(listing_summary.listing.shipping_info.as_ref().map(|si| serde_json::to_value(si).unwrap_or(serde_json::Value::Null)))
        .bind(&listing_summary.listing.condition_details)
        .bind(&listing_summary.listing.seller_notes)
        .bind(db_enum_value(&listing_summary.listing.listing_type))
        .bind(listing_summary.listing.location.latitude)
        .bind(listing_summary.listing.location.longitude)
        .bind(listing_summary.listing.location.geolocation_opt_out)
        .execute(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;

        Ok(listing_summary)
    }

    async fn get_listing(
        &self,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let row = sqlx::query(
            "SELECT l.listing_id, l.owner_id, l.schema_version, l.category, l.product_name, l.\"condition\", l.price_currency, l.price_amount::TEXT AS price_amount, l.country_code, l.country_name, l.city, l.picture_urls, l.description, l.attributes, l.status, l.version, l.sku, l.quantity, l.shipping_info, l.condition_details, l.seller_notes, l.listing_type, l.latitude, l.longitude, l.geolocation_opt_out,
                s.display_name, s.seller_rating, s.verified_at
              FROM listings l
              LEFT JOIN seller_accounts s ON l.owner_id = s.owner_id
              WHERE l.listing_id = $1",
        )
        .bind(listing_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;

        if let Some(row) = row {
            Ok(Some(row_to_summary(row)?))
        } else {
            Ok(None)
        }
    }

    async fn search_listings(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, RepositoryError> {
        let mut items = self.fetch_rows(request).await?;
        let limit = request.limit.unwrap_or(50).min(200) as usize;

        // Cursor-based pagination: skip items before (and including) the cursor
        if let Some(ref cursor) = request.cursor {
            if let Some(pos) = items.iter().position(|item| item.listing_id == *cursor) {
                items = items.into_iter().skip(pos + 1).collect();
            }
        }

        let next_cursor = if items.len() > limit {
            Some(items[limit - 1].listing_id.clone())
        } else {
            None
        };
        items.truncate(limit);

        Ok(SearchResponse {
            items,
            applied_sort_by: request.sort_by,
            next_cursor,
        })
    }

    async fn update_listing_status(
        &self,
        listing_id: &str,
        status: ListingStatus,
    ) -> Result<Option<ListingSummary>, RepositoryError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;

        let row = sqlx::query(
            "UPDATE listings SET status = $1, version = version + 1, updated_at = now() WHERE listing_id = $2 RETURNING listing_id, owner_id, schema_version, category, product_name, \"condition\", price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls, description, attributes, status, version, sku, quantity, shipping_info, condition_details, seller_notes, listing_type, latitude, longitude, geolocation_opt_out",
        )
        .bind(db_enum_value(&status))
        .bind(listing_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;

        if let Some(row) = row {
            Ok(Some(row_to_summary(row)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::*;

    fn make_test_request(title: &str, owner_id: &str) -> CreateListingRequest {
        CreateListingRequest {
            idempotency_key: format!("idem_{}", title.len()),
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: owner_id.to_string(),
                listing_type: ListingType::Product,
                category: Some(Category::Laptop),
                title: title.to_string(),
                condition: Some(Condition::New),
                price: Price {
                    amount: 999.99,
                    currency: "USD".to_string(),
                },
                location: ListingLocation {
                    country_code: "US".to_string(),
                    country_name: "United States".to_string(),
                    city: "New York".to_string(),
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec!["http://example.com/img.jpg".to_string()],
                description: "Test description".to_string(),
                attributes: None,
                sku: Some("SKU123".to_string()),
                quantity: Some(10),
                shipping_info: None,
                condition_details: None,
                seller_notes: None,
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
            },
        }
    }

    #[tokio::test]
    async fn insert_listing_creates_new_listing() {
        let repo = InMemoryListingRepository::new();
        let request = make_test_request("MacBook Pro", "seller_1");

        let result = repo.insert_listing(&request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.listing_id.starts_with("lst_"));
        assert_eq!(response.listing.title, "MacBook Pro");
        assert_eq!(response.status, ListingStatus::Active);
    }

    #[tokio::test]
    async fn insert_listing_rejects_duplicate() {
        let repo = InMemoryListingRepository::new();
        let request = make_test_request("MacBook Pro", "seller_1");

        let first = repo.insert_listing(&request).await;
        assert!(first.is_ok());

        let second = repo.insert_listing(&request).await;
        assert!(second.is_err());
    }

    #[tokio::test]
    async fn get_listing_returns_inserted_listing() {
        let repo = InMemoryListingRepository::new();
        let request = make_test_request("Test Item", "seller_1");

        let inserted = repo.insert_listing(&request).await.unwrap();
        let retrieved = repo.get_listing(&inserted.listing_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().listing_id, inserted.listing_id);
    }

    #[tokio::test]
    async fn get_listing_returns_none_for_missing() {
        let repo = InMemoryListingRepository::new();
        let result = repo.get_listing("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn search_listings_filters_by_category() {
        let repo = InMemoryListingRepository::new();

        let mut laptop_req = make_test_request("Laptop", "seller_1");
        let mut phone_req = make_test_request("Phone", "seller_1");
        laptop_req.listing.category = Some(Category::Laptop);
        phone_req.listing.category = Some(Category::Phone);

        repo.insert_listing(&laptop_req).await.unwrap();
        repo.insert_listing(&phone_req).await.unwrap();

        let search_req = SearchRequest {
            category: Some(Category::Laptop),
            ..Default::default()
        };
        let results = repo.search_listings(&search_req).await.unwrap();

        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].listing.category, Some(Category::Laptop));
    }

    #[tokio::test]
    async fn search_listings_respects_limit() {
        let repo = InMemoryListingRepository::new();

        for i in 0..25 {
            let req = make_test_request(&format!("Item {}", i), "seller_1");
            repo.insert_listing(&req).await.unwrap();
        }

        let search_req = SearchRequest {
            limit: Some(5),
            ..Default::default()
        };
        let results = repo.search_listings(&search_req).await.unwrap();

        assert_eq!(results.items.len(), 5);
    }

    #[tokio::test]
    async fn search_listings_sorts_by_price_asc() {
        let repo = InMemoryListingRepository::new();

        let mut req1 = make_test_request("Expensive Item", "seller_1");
        req1.listing.price.amount = 1000.0;
        let mut req2 = make_test_request("Cheap Item", "seller_1");
        req2.listing.price.amount = 100.0;

        repo.insert_listing(&req1).await.unwrap();
        repo.insert_listing(&req2).await.unwrap();

        let search_req = SearchRequest {
            sort_by: SearchSort::PriceAsc,
            ..Default::default()
        };
        let results = repo.search_listings(&search_req).await.unwrap();

        assert_eq!(results.items.len(), 2);
        assert_eq!(results.items[0].listing.price.amount, 100.0);
    }

    #[tokio::test]
    async fn search_listings_filters_by_text_query() {
        let repo = InMemoryListingRepository::new();

        let mut req1 = make_test_request("MacBook Pro 16 inch", "seller_1");
        req1.listing.description = "Powerful laptop".to_string();
        let mut req2 = make_test_request("Old Phone", "seller_1");
        req2.listing.description = "Basic phone".to_string();

        repo.insert_listing(&req1).await.unwrap();
        repo.insert_listing(&req2).await.unwrap();

        let search_req = SearchRequest {
            query: Some("macbook".to_string()),
            ..Default::default()
        };
        let results = repo.search_listings(&search_req).await.unwrap();

        // Query is used for scoring/sorting but not for filtering in in-memory repo
        assert_eq!(results.items.len(), 2);
        // MacBook should be first due to relevance score
        assert!(results.items[0].listing.title.contains("MacBook"));
    }

    #[tokio::test]
    async fn search_listings_returns_applied_sort() {
        let repo = InMemoryListingRepository::new();
        let req = make_test_request("Test", "seller_1");
        repo.insert_listing(&req).await.unwrap();

        let search_req = SearchRequest {
            sort_by: SearchSort::PriceDesc,
            ..Default::default()
        };
        let results = repo.search_listings(&search_req).await.unwrap();

        assert_eq!(results.applied_sort_by, SearchSort::PriceDesc);
    }

    // ------------------------------------------------------------------
    // Priority 3.5: Concurrent access safety
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn concurrent_inserts_all_succeed() {
        let repo = Arc::new(InMemoryListingRepository::new());
        let mut handles = Vec::new();
        for i in 0..20u64 {
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let req = make_test_request(&format!("Item {i}"), &format!("seller_{i}"));
                repo.insert_listing(&req).await
            }));
        }
        let mut successes: usize = 0;
        for handle in handles {
            let result = handle.await.unwrap();
            if result.is_ok() {
                successes += 1;
            }
        }
        assert_eq!(successes, 20);
    }

    #[tokio::test]
    async fn concurrent_insert_and_read_no_panic() {
        let repo = Arc::new(InMemoryListingRepository::new());
        let req = make_test_request("Concurrent", "seller_1");
        let created = repo.insert_listing(&req).await.unwrap();
        let listing_id = created.listing_id.clone();

        let mut handles = Vec::new();
        for _ in 0..10 {
            let repo = Arc::clone(&repo);
            let lid = listing_id.clone();
            handles.push(tokio::spawn(async move {
                let _ = repo.get_listing(&lid).await;
                let _ = repo.search_listings(&SearchRequest::default()).await;
            }));
        }
        for _ in 0..10 {
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let req = make_test_request("New", "seller_2");
                let _ = repo.insert_listing(&req).await;
            }));
        }
        for handle in handles {
            assert!(handle.await.is_ok());
        }
    }
}
