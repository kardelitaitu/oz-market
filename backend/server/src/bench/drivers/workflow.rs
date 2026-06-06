use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use oz_market_api_contract::*;
use oz_market_auth_core::Claims;

use crate::app::MarketplaceApp;
use crate::models::db::SellerAccountRow;
use crate::repositories::*;
use crate::repositories::audit_events::InMemoryAuditEventRepository;
use crate::repositories::outbox_events::InMemoryOutboxEventRepository;
use crate::services::idempotency::InMemoryIdempotencyRepository;

use super::super::driver::{BenchError, BenchmarkDriver};

type InMemoryApp = MarketplaceApp<
    InMemoryListingRepository,
    InMemoryIdempotencyRepository,
    InMemoryReservationLeaseRepository,
    InMemoryContactRevealRepository,
>;

struct WorkflowState {
    app: InMemoryApp,
    seller_claims: Claims,
    buyer_claims: Claims,
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Benchmark driver that exercises the full business workflow cycle:
/// search (buyer) → open negotiation (buyer) → request contact reveal
/// (buyer) → approve contact reveal (seller).
///
/// Uses in-memory repositories so it needs no external database.
pub struct WorkflowDriver {
    state: Mutex<Option<WorkflowState>>,
    cursor: AtomicUsize,
}

impl WorkflowDriver {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
            cursor: AtomicUsize::new(0),
        }
    }

    fn fingerprint(&self, label: &str, n: usize) -> String {
        format!("wf-{label}-{n}")
    }
}

impl Default for WorkflowDriver {
    fn default() -> Self {
        Self::new()
    }
}

fn make_listing(seed: usize) -> CreateListingRequest {
    CreateListingRequest {
        idempotency_key: format!("seed-{seed}"),
        listing: ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "bench-seller".to_string(),
            listing_type: ListingType::Product,
            category: Some(Category::Other),
            title: format!("Workflow Benchmark Product {seed}"),
            condition: Some(Condition::New),
            price: Price {
                currency: "USD".to_string(),
                amount: 49.99 + seed as f64,
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "Benchmark City".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/bench.jpg".to_string()],
            description: format!("Workflow benchmark product #{seed}"),
            attributes: None,
            sku: Some(format!("WF-{seed}")),
            quantity: Some(1),
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

#[async_trait]
impl BenchmarkDriver for WorkflowDriver {
    async fn setup(&self) -> Result<(), BenchError> {
        let now = now_rfc3339();

        let seller_account_repo = InMemorySellerAccountRepository::new();
        seller_account_repo.add_account(SellerAccountRow {
            seller_account_id: "bench-seller-id".to_string(),
            owner_id: "bench-seller".to_string(),
            display_name: None,
            trust_level: "trusted".to_string(),
            seller_rating: None,
            quota_override: Some(1000),
            listings_created: 0,
            status: "active".to_string(),
            hardware_fingerprint: None,
            verified_at: Some(now.clone()),
            created_at: now.clone(),
            updated_at: now.clone(),
        });

        let app = InMemoryApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(seller_account_repo),
        );

        let seller_claims = Claims {
            sub: "bench-seller".to_string(),
            seller_account_id: Some("bench-seller-id".to_string()),
            ..Claims::default()
        };

        let buyer_claims = Claims {
            sub: "bench-buyer".to_string(),
            buyer_agent_id: Some("bench-buyer".to_string()),
            ..Claims::default()
        };

        for i in 0..100 {
            let req = make_listing(i);
            app.create_listing(&seller_claims, &req, &format!("setup-seed-{i}"), &now)
                .await
                .map_err(|e| BenchError::Execution(format!("seed failed: {e}")))?;
        }

        *self.state.lock().await = Some(WorkflowState {
            app,
            seller_claims,
            buyer_claims,
        });

        Ok(())
    }

    async fn run_operation(&self) -> Result<Duration, BenchError> {
        let start = std::time::Instant::now();

        let mut guard = self.state.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| BenchError::Execution("driver not set up".to_string()))?;

        let n = self.cursor.fetch_add(1, Ordering::SeqCst);
        let now = now_rfc3339();

        let search_resp: SearchResponse = state
            .app
            .search_listings(Some(&state.buyer_claims), &SearchRequest {
                query: Some("Workflow".to_string()),
                limit: Some(5),
                ..Default::default()
            })
            .await
            .map_err(|e| BenchError::Execution(format!("search failed: {e}")))?;

        let listing_id = search_resp
            .items
            .first()
            .map(|item| item.listing_id.clone())
            .ok_or_else(|| BenchError::Execution("no listings found".to_string()))?;

        let open_req = OpenNegotiationRequest {
            listing_id: listing_id.clone(),
            buyer_agent_id: "bench-buyer".to_string(),
            offer_currency: "USD".to_string(),
            offer_amount: 49.99,
            idempotency_key: format!("wf-open-{n}"),
        };

        let (negotiation, _) = state
            .app
            .open_negotiation(
                &state.buyer_claims,
                &open_req,
                &self.fingerprint("open", n),
                &now,
            )
            .await
            .map_err(|e| BenchError::Execution(format!("open negotiation failed: {e}")))?;

        let reveal_req = RequestContactRevealRequest {
            idempotency_key: format!("wf-reveal-req-{n}"),
        };

        let reveal = state
            .app
            .request_contact_reveal(
                &state.buyer_claims,
                &negotiation.negotiation_id,
                &reveal_req,
                &self.fingerprint("reveal-req", n),
                &now,
            )
            .await
            .map_err(|e| BenchError::Execution(format!("request reveal failed: {e}")))?;

        state
            .app
            .approve_contact_reveal(&state.seller_claims, &reveal.reveal_id)
            .await
            .map_err(|e| BenchError::Execution(format!("approve reveal failed: {e}")))?;

        Ok(start.elapsed())
    }

    async fn teardown(&self) -> Result<(), BenchError> {
        *self.state.lock().await = None;
        Ok(())
    }
}
