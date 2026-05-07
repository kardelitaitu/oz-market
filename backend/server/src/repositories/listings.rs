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
            category: summary.listing.category,
            product_name: summary.listing.product_name.clone(),
            item_condition: summary.listing.condition,
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
            shipping_info: summary.listing.shipping_info.as_ref().map(|si| serde_json::to_value(si).unwrap_or(serde_json::Value::Null)),
            condition_details: summary.listing.condition_details.clone(),
            seller_notes: summary.listing.seller_notes.clone(),
            status: summary.status,
            version: summary.version as i64,
            create_idempotency_key: String::new(),
            search_text: crate::services::search::listing_index_text(&summary.listing),
            created_at: String::new(),
            updated_at: String::new(),
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
                && listing.listing.product_name == request.listing.product_name
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

    Ok(ListingSummary {
        listing_id,
        status,
        version,
        listing: marketplace_api_contract::ListingPayload {
            schema_version,
            owner_id: row
                .try_get::<String, _>("owner_id")
                .map_err(|error| storage(error.to_string()))?,
            category,
            product_name,
            condition,
            price: marketplace_api_contract::Price {
                currency: price_currency,
                amount: price_amount,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code,
                country_name,
                city,
            },
            picture_urls,
            description,
            attributes,
            // NEW: Marketplace fields
            sku,
            quantity: if quantity == 1 { None } else { Some(quantity as u32) },
            shipping_info: shipping_info.and_then(|v| serde_json::from_value(v).ok()),
            condition_details,
            seller_notes,
        },
        // Seller fields (read-only, None for now)
        seller_name: None,
        seller_rating: None,
        seller_verified: None,
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
            "SELECT listing_id, owner_id, schema_version, category, product_name, \"condition\", price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls, description, attributes, status, version, sku, quantity, shipping_info, condition_details, seller_notes FROM listings",
        );
        let mut where_added = false;
        
        if let Some(category) = request.category {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("category = ").push_bind(db_enum_value(&category));
        }
        if let Some(condition) = request.condition {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("\"condition\" = ").push_bind(db_enum_value(&condition));
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
                builder.push("LOWER(city) = ").push_bind(city.to_ascii_lowercase());
            }
        }
        if let Some(query) = &request.query {
            if where_added {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                where_added = true;
            }
            builder.push("search_text LIKE ").push_bind(format!("%{}%", query.to_ascii_lowercase()));
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
                search_text, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'active',1,$15,$16,now(),now())
            RETURNING listing_id, owner_id, schema_version, category, product_name, \"condition\",
                price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls,
                description, attributes, status, version",
        )
        .bind(&listing_id)
        .bind(&request.listing.owner_id)
        .bind(&request.listing.schema_version)
        .bind(db_enum_value(&request.listing.category))
        .bind(&request.listing.product_name)
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
        .fetch_one(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;
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
            "SELECT listing_id, owner_id, schema_version, category, product_name, \"condition\",
                price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls,
                description, attributes, status, version
             FROM listings
             WHERE listing_id = $1",
        )
        .bind(listing_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|error| storage(error.to_string()))?;
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
            "UPDATE listings SET status = $1, version = version + 1, updated_at = now() WHERE listing_id = $2 RETURNING listing_id, owner_id, schema_version, category, product_name, \"condition\", price_currency, price_amount::TEXT AS price_amount, country_code, country_name, city, picture_urls, description, attributes, status, version",
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
        if listing.listing.category != category {
            return false;
        }
    }

    if let Some(condition) = request.condition {
        if listing.listing.condition != condition {
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
                category: Category::Laptop,
                product_name: product_name.to_string(),
                condition: Condition::Used,
                price: Price {
                    currency: "USD".to_string(),
                    amount,
                },
                location: ListingLocation {
                    country_code: "JP".to_string(),
                    country_name: "Japan".to_string(),
                    city: city.to_string(),
                },
                picture_urls: vec!["https://example.com/item.jpg".to_string()],
                description: format!("{product_name} in {city}"),
                attributes: Some(
                    [("brand".to_string(), json!("Lenovo"))]
                        .into_iter()
                        .collect(),
                ),
                // NEW: Marketplace fields
                sku: None,
                quantity: None,
                shipping_info: None,
                condition_details: None,
                seller_notes: None,
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
            .all(|item| item.listing.product_name.contains("ThinkPad")));
    }
}
