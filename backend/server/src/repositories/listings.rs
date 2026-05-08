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

pub fn storage(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Storage, message)
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
            quantity: summary.listing.quantity.map(|q| q as i32).unwrap_or(1),
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
        .try_get::<i32, _>("quantity")
        .map_err(|error| storage(error.to_string()))?;
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
        if let Some(price) = &request.price {
            if let Some(currency) = &price.currency {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_currency = ").push_bind(currency);
            }
            if let Some(min_amount) = price.min_amount {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_amount >= ").push_bind(min_amount);
            }
            if let Some(max_amount) = price.max_amount {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("price_amount <= ").push_bind(max_amount);
            }
        }
        if let Some(location) = &request.location {
            if let Some(country_code) = &location.country_code {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder.push("country_code = ").push_bind(country_code);
            }
            if let Some(city) = &location.city {
                if where_added {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    where_added = true;
                }
                builder
                    .push("LOWER(city) = ")
                    .push_bind(city.to_ascii_lowercase());
            }
        }
        if let Some(query) = &request.query {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            // Phase C: Check for "seller:" prefix (case-insensitive)
            if query.to_lowercase().starts_with("seller:") {
                // Extract seller name after "seller:" prefix
                let seller_query = query.trim_start_matches("seller:").trim();
                builder
                    .push("s.display_name ILIKE ")
                    .push_bind(format!("%{}%", seller_query));
            } else {
                // Normal search in search_text
                builder
                    .push("search_text LIKE ")
                    .push_bind(format!("%{}%", query.to_ascii_lowercase()));
            }
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
                .push("listing_type = ")
                .push_bind(db_enum_value(&listing_type));

            // JOIN with separate table based on listing_type
            if matches!(listing_type, marketplace_api_contract::ListingType::Service) {
                builder.push(" LEFT JOIN service_listings sl ON l.listing_id = sl.listing_id");
            } else if matches!(
                listing_type,
                marketplace_api_contract::ListingType::Property
            ) {
                builder.push(" LEFT JOIN property_listings pl ON l.listing_id = pl.listing_id");
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
                let radius_km = request.radius_km.unwrap_or(50.0); // Default: 50km

                builder.push("AND (");
                builder.push("  6371 * acos("); // Earth's radius in km
                builder
                    .push("    cos(radians(")
                    .push_bind(user_lat)
                    .push(")) * ");
                builder.push("    cos(radians(l.latitude)) * ");
                builder
                    .push("    cos(radians(l.longitude) - radians(")
                    .push_bind(user_lon)
                    .push(")) + ");
                builder
                    .push("    sin(radians(")
                    .push_bind(user_lat)
                    .push(")) * ");
                builder.push("    sin(radians(l.latitude))");
                builder.push("  ) <= ").push_bind(radius_km);
                builder.push(")");

                // Order by distance (nearest first) - compute inline
                builder.push(" ORDER BY ");
                builder.push("  6371 * acos(");
                builder
                    .push("    cos(radians(")
                    .push_bind(user_lat)
                    .push(")) * ");
                builder.push("    cos(radians(l.latitude)) * ");
                builder
                    .push("    cos(radians(l.longitude) - radians(")
                    .push_bind(user_lon)
                    .push(")) + ");
                builder
                    .push("    sin(radians(")
                    .push_bind(user_lat)
                    .push(")) * ");
                builder.push("    sin(radians(l.latitude))");
                builder.push("  ) ASC, l.listing_id");
            } else {
                // User location not provided - fall back to city/country match
                // (already implemented via existing location filter)
            }
        }

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

        if let Some(cursor) = request.cursor.as_deref() {
            if let Some(index) = items.iter().position(|item| item.listing_id == cursor) {
                items = items.into_iter().skip(index + 1).collect();
            }
        }

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
        let next_id: i64 = sqlx::query_scalar("SELECT nextval('listing_id_seq')")
            .fetch_one(&mut *conn)
            .await
            .map_err(|error| storage(error.to_string()))?;
        let listing_id = format!("lst_{next_id:06}");
        let row = sqlx::query(
            "INSERT INTO listings (
                listing_id, owner_id, schema_version, category, product_name, \"condition\",
                price_currency, price_amount, country_code, country_name, city,
                picture_urls, description, attributes, status, version, create_idempotency_key,
                search_text, created_at, updated_at, sku, quantity, shipping_info, condition_details, seller_notes, listing_type
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'active',1,$15,$16,now(),now(),$17,$18,$19,$20,$22)
            RETURNING listing_id, owner_id, schema_version, category, product_name, \"condition\",
                price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls,
                description, attributes, status, version, sku, quantity, shipping_info, condition_details, seller_notes, listing_type",
        )
        .bind(&listing_id)
        .bind(&request.listing.owner_id)
        .bind(&request.listing.schema_version)
        .bind(db_enum_value(&request.listing.category))
        .bind(&request.listing.title)
        .bind(db_enum_value(&request.listing.condition))
        .bind(&request.listing.price.currency)
        .bind(request.listing.price.amount)
        .bind(&request.listing.location.country_code)
        .bind(&request.listing.location.country_name)
        .bind(&request.listing.location.city)
        .bind(Json(&request.listing.picture_urls))
        .bind(&request.listing.description)
        .bind(Json(&request.listing.attributes))
        .bind(&request.idempotency_key)
        .bind(crate::services::search::listing_index_text(&request.listing).to_ascii_lowercase())
        .bind(&request.listing.sku)
        .bind(request.listing.quantity.unwrap_or(1) as i32)
        .bind(request.listing.shipping_info.as_ref().map(|si| serde_json::to_value(si).unwrap_or(serde_json::Value::Null)))
        .bind(&request.listing.condition_details)
        .bind(&request.listing.seller_notes)
        .bind(db_enum_value(&request.listing.listing_type))
        .fetch_one(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;

        // NEW: Phase 4 - Insert into separate table based on listing_type
        match request.listing.listing_type {
            marketplace_api_contract::ListingType::Service => {
                sqlx::query(
                    "INSERT INTO service_listings (listing_id, service_type, hourly_rate, project_rate, qualifications, service_radius_km) 
                     VALUES ($1, $2, $3, $4, $5, $6)"
                )
                .bind(&listing_id)
                .bind(db_enum_value(&request.listing.service_type))
                .bind(request.listing.hourly_rate)
                .bind(request.listing.project_rate)
                .bind(Json(&request.listing.qualifications))
                .bind(request.listing.service_radius_km)
                .execute(&mut *conn)
                .await
                .map_err(|error| storage(error.to_string()))?;
            }
            marketplace_api_contract::ListingType::Property => {
                sqlx::query(
                    "INSERT INTO property_listings (listing_id, property_transaction_type, property_sub_type, area_sqm, bedrooms, bathrooms, year_built, lot_size_sqm, zoning) 
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
                )
                .bind(&listing_id)
                .bind(db_enum_value(&request.listing.property_transaction_type))
                .bind(db_enum_value(&request.listing.property_sub_type))
                .bind(request.listing.area_sqm)
                .bind(request.listing.bedrooms)
                .bind(request.listing.bathrooms)
                .bind(request.listing.year_built)
                .bind(request.listing.lot_size_sqm)
                .bind(&request.listing.zoning)
                .execute(&mut *conn)
                .await
                .map_err(|error| storage(error.to_string()))?;
            }
            _ => {} // Product - no separate table needed
        }

        row_to_summary(row)
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
            "SELECT l.listing_id, l.owner_id, l.schema_version, l.category, l.product_name, l.\"condition\",
                l.price_currency, l.price_amount::TEXT AS price_amount, l.country_code, l.country_name, l.city, l.picture_urls,
                l.description, l.attributes, l.status, l.version, l.sku, l.quantity, l.shipping_info, l.condition_details, l.seller_notes,
                l.listing_type, l.latitude, l.longitude, l.geolocation_opt_out,
                s.display_name, s.seller_rating, s.verified_at
             FROM listings l
             LEFT JOIN seller_accounts s ON l.owner_id = s.owner_id
             WHERE l.listing_id = $1",
        )
        .bind(listing_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;

        if let Some(row) = &row {
            // NEW: Phase 4 - Fetch from separate table based on listing_type
            let listing_type_str: String = row
                .try_get("listing_type")
                .ok()
                .unwrap_or_else(|| "product".to_string());

            if listing_type_str == "service" {
                // Fetch from service_listings
                let _service_row = sqlx::query(
                    "SELECT service_type, hourly_rate, project_rate, qualifications, service_radius_km FROM service_listings WHERE listing_id = $1"
                )
                .bind(listing_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(|error| storage(error.to_string()))?;

                // TODO: Merge service data into summary
                // For now, just return base summary
            } else if listing_type_str == "property" {
                // Fetch from property_listings
                let _property_row = sqlx::query(
                    "SELECT property_transaction_type, property_sub_type, area_sqm, bedrooms, bathrooms, year_built, lot_size_sqm, zoning 
                     FROM property_listings WHERE listing_id = $1"
                )
                .bind(listing_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(|error| storage(error.to_string()))?;

                // TODO: Merge property data into summary
            }
        }

        row.map(row_to_summary).transpose()
    }

    async fn search_listings(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchResponse, RepositoryError> {
        let mut items = self.fetch_rows(request).await?;
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
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|error| storage(error.to_string()))?;
        let rows = sqlx::query(
            "UPDATE listings SET status = $1, version = version + 1, updated_at = now() WHERE listing_id = $2 RETURNING listing_id, owner_id, schema_version, category, product_name, \"condition\", price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls, description, attributes, status, version, sku, quantity, shipping_info, condition_details, seller_notes, latitude, longitude, geolocation_opt_out",
        )
        .bind(db_enum_value(&status))
        .bind(listing_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;
        Ok(rows.map(row_to_summary).transpose()?)
    }
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

    if let Some(price) = &request.price {
        if let Some(currency) = &price.currency {
            if &listing.listing.price.currency != currency {
                return false;
            }
        }
        if let Some(min_amount) = price.min_amount {
            if listing.listing.price.amount < min_amount {
                return false;
            }
        }
        if let Some(max_amount) = price.max_amount {
            if listing.listing.price.amount > max_amount {
                return false;
            }
        }
    }

    if let Some(location) = &request.location {
        if let Some(country_code) = &location.country_code {
            if &listing.listing.location.country_code != country_code {
                return false;
            }
        }
        if let Some(city) = &location.city {
            if !listing.listing.location.city.eq_ignore_ascii_case(city) {
                return false;
            }
        }
    }

    if let Some(query) = &request.query {
        let terms = crate::services::search::normalize_search_terms(query);
        let score = crate::services::search::score_listing(listing, &terms);
        if score == 0 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::{
        Category, Condition, CreateListingRequest, ListingLocation, ListingPayload, Price,
        SearchRequest, SearchSort,
    };
    use serde_json::json;

    fn build_request(
        owner_id: &str,
        product_name: &str,
        amount: f64,
        city: &str,
    ) -> CreateListingRequest {
        CreateListingRequest {
            idempotency_key: format!("idem-{owner_id}-{product_name}"),
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: owner_id.to_string(),
                listing_type: marketplace_api_contract::ListingType::Product,
                category: Some(Category::Laptop),
                title: product_name.to_string(),
                condition: Some(Condition::Used),
                price: Price {
                    currency: "USD".to_string(),
                    amount,
                },
                location: ListingLocation {
                    country_code: "JP".to_string(),
                    country_name: "Japan".to_string(),
                    city: city.to_string(),
                    // Phase D: Geolocation (optional)
                    latitude: None,
                    longitude: None,
                    geolocation_opt_out: None,
                },
                picture_urls: vec!["https://example.com/item.jpg".to_string()],
                description: format!("{product_name} in {city}"),
                attributes: Some(
                    [("brand".to_string(), json!("Lenovo"))]
                        .into_iter()
                        .collect(),
                ),
                // Marketplace fields
                sku: None,
                quantity: None,
                shipping_info: None,
                condition_details: None,
                seller_notes: None,
                // Phase 4: Service fields (None for Product)
                service_type: None,
                hourly_rate: None,
                project_rate: None,
                qualifications: None,
                service_radius_km: None,
                // Phase 4: Property fields (None for Product)
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
    async fn search_is_deterministic_and_filtered() {
        let repo = InMemoryListingRepository::new();
        let first = repo
            .insert_listing(&build_request("seller-1", "ThinkPad T480", 450.0, "Osaka"))
            .await
            .unwrap();
        let _ = repo
            .insert_listing(&build_request("seller-2", "ThinkPad X1", 900.0, "Tokyo"))
            .await
            .unwrap();
        let _ = repo
            .insert_listing(&build_request("seller-3", "MacBook Air", 1200.0, "Osaka"))
            .await
            .unwrap();

        let response = repo
            .search_listings(&SearchRequest {
                query: Some("ThinkPad".to_string()),
                category: Some(Category::Laptop),
                condition: Some(Condition::Used),
                sort_by: SearchSort::Relevance,
                limit: Some(10),
                ..SearchRequest::default()
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].listing_id, first.listing_id);
        assert!(response
            .items
            .iter()
            .all(|item| item.listing.title.contains("ThinkPad")));
    }
}
