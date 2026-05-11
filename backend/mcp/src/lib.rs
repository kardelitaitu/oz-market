//! MCP Server for Marketplace
//!
//! Implements Model Context Protocol (MCP) to let AI agents interact
//! with the marketplace via standardized tools.

use marketplace_api_contract::{
    AcceptNegotiationRequest, Category, Condition, ContactRevealResponse, CreateListingRequest,
    CreateListingResponse, ListingSummary, NegotiationResponse, OpenNegotiationRequest,
    RejectNegotiationRequest, RequestContactRevealRequest, SearchRequest, SearchResponse,
    SubmitOfferRequest,
};
use marketplace_auth_core::Claims;
use marketplace_server::app::MarketplaceApp;

type InMemoryApp = MarketplaceApp<
    marketplace_server::repositories::listings::InMemoryListingRepository,
    marketplace_server::services::idempotency::InMemoryIdempotencyRepository,
    marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository,
    marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository,
>;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app = InMemoryApp::new(
        marketplace_server::repositories::listings::InMemoryListingRepository::new(),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository::new(),
        marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository::new(),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::InMemoryNegotiationRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository::new(
            ),
        ),
    );
    let _mcp = MarketplaceMcp::new(app);
    Ok(())
}

pub struct MarketplaceMcp {
    app: InMemoryApp,
}

impl MarketplaceMcp {
    pub fn new(app: InMemoryApp) -> Self {
        Self { app }
    }

    pub async fn search_listings(
        &self,
        claims: &Claims,
        request: &SearchRequest,
    ) -> Result<SearchResponse, marketplace_server::http::handlers::HandlerError> {
        self.app.search_listings(Some(claims), request).await
    }

    pub async fn get_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, marketplace_server::http::handlers::HandlerError> {
        self.app.get_listing(Some(claims), listing_id).await
    }

    pub async fn begin_create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<
        marketplace_server::services::idempotency::IdempotencyDecision,
        marketplace_server::http::handlers::HandlerError,
    > {
        self.app
            .begin_create_listing(claims, request, request_fingerprint, now_rfc3339)
            .await
    }

    pub async fn create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<CreateListingResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .create_listing(claims, request, request_fingerprint, now_rfc3339)
            .await
    }

    pub async fn begin_open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<
        marketplace_server::services::idempotency::IdempotencyDecision,
        marketplace_server::http::handlers::HandlerError,
    > {
        self.app
            .begin_open_negotiation(claims, request, request_fingerprint, now_rfc3339)
            .await
    }

    pub async fn open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .open_negotiation(claims, request, request_fingerprint, now_rfc3339)
            .await
    }

    pub async fn get_negotiation_status(
        &self,
        claims: &Claims,
        negotiation_id: &str,
    ) -> Result<NegotiationResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .get_negotiation_status(claims, negotiation_id)
            .await
    }

    pub async fn submit_offer(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &SubmitOfferRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .submit_offer(
                claims,
                negotiation_id,
                request,
                request_fingerprint,
                now_rfc3339,
            )
            .await
    }

    pub async fn accept_negotiation(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &AcceptNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .accept_negotiation(
                claims,
                negotiation_id,
                request,
                request_fingerprint,
                now_rfc3339,
            )
            .await
    }

    pub async fn reject_negotiation(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &RejectNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .reject_negotiation(
                claims,
                negotiation_id,
                request,
                request_fingerprint,
                now_rfc3339,
            )
            .await
    }

    pub async fn request_contact_reveal(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, marketplace_server::http::handlers::HandlerError> {
        self.app
            .request_contact_reveal(
                claims,
                negotiation_id,
                request,
                request_fingerprint,
                now_rfc3339,
            )
            .await
    }

    pub async fn approve_contact_reveal(
        &self,
        claims: &Claims,
        reveal_id: &str,
    ) -> Result<ContactRevealResponse, marketplace_server::http::handlers::HandlerError> {
        self.app.approve_contact_reveal(claims, reveal_id).await
    }

    pub async fn get_contact_reveal(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, marketplace_server::http::handlers::HandlerError>
    {
        self.app.get_contact_reveal(reveal_id).await
    }

    pub async fn archive_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ListingSummary>, marketplace_server::http::handlers::HandlerError> {
        self.app
            .archive_listing(claims, listing_id, reason, now_rfc3339)
            .await
    }

    pub async fn set_seller_trust_level(
        &self,
        claims: &Claims,
        seller_account_id: &str,
        trust_level: &str,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<
        Option<marketplace_server::models::db::SellerAccountRow>,
        marketplace_server::http::handlers::HandlerError,
    > {
        self.app
            .set_seller_trust_level(claims, seller_account_id, trust_level, reason, now_rfc3339)
            .await
    }

    pub async fn set_seller_quota_override(
        &self,
        claims: &Claims,
        seller_account_id: &str,
        quota_override: Option<i32>,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<
        Option<marketplace_server::models::db::SellerAccountRow>,
        marketplace_server::http::handlers::HandlerError,
    > {
        self.app
            .set_seller_quota_override(
                claims,
                seller_account_id,
                quota_override,
                reason,
                now_rfc3339,
            )
            .await
    }
}

#[allow(dead_code)]
fn build_claims() -> Claims {
    Claims {
        sub: "sub-1".to_string(),
        roles: vec![
            marketplace_auth_core::Role::SellerListingWriter,
            marketplace_auth_core::Role::BuyerNegotiator,
        ],
        scopes: vec![
            marketplace_auth_core::Scope::ListingCreate,
            marketplace_auth_core::Scope::ListingRead,
            marketplace_auth_core::Scope::ListingSearch,
            marketplace_auth_core::Scope::NegotiationCreate,
            marketplace_auth_core::Scope::NegotiationRevealRequest,
            marketplace_auth_core::Scope::RevealApprove,
        ],
        seller_account_id: Some("seller-1".to_string()),
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: None,
    }
}

#[allow(dead_code)]
fn build_admin_claims() -> Claims {
    Claims {
        sub: "admin-1".to_string(),
        roles: vec![marketplace_auth_core::Role::Admin],
        scopes: vec![
            marketplace_auth_core::Scope::ListingRead,
            marketplace_auth_core::Scope::NegotiationRead,
            marketplace_auth_core::Scope::RevealApprove,
        ],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

#[allow(dead_code)]
fn build_create_request() -> CreateListingRequest {
    CreateListingRequest {
        idempotency_key: "idem-create-1".to_string(),
        listing: marketplace_api_contract::ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "seller-1".to_string(),
            category: Some(Category::Laptop),
            title: "ThinkPad T480".to_string(),
            condition: Some(Condition::Used),
            price: marketplace_api_contract::Price {
                currency: "USD".to_string(),
                amount: 450.0,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: "JP".to_string(),
                country_name: "Japan".to_string(),
                city: "Osaka".to_string(),
                // Phase D: Geolocation (optional)
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/item.jpg".to_string()],
            description: "Good battery health".to_string(),
            attributes: None,
            // NEW: Marketplace fields
            sku: None,
            quantity: None,
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            // NEW: Phase 2 fields
            listing_type: marketplace_api_contract::ListingType::Product,
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

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::SearchSort;
    use marketplace_server::repositories::audit_events::InMemoryAuditEventRepository;
    use marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository;
    use marketplace_server::repositories::listings::InMemoryListingRepository;
    use marketplace_server::repositories::negotiations::InMemoryNegotiationRepository;
    use marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository;
    use marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository;
    use marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository;
    use marketplace_server::services::idempotency::InMemoryIdempotencyRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn mcp_delegates_search_and_idempotency_to_shared_app() {
        let listing_repo = InMemoryListingRepository::new();
        let idempotency_repo = InMemoryIdempotencyRepository::new();
        let app = MarketplaceApp::new(
            listing_repo,
            idempotency_repo,
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let mcp = MarketplaceMcp::new(app);
        let claims = build_claims();
        let request = build_create_request();

        let created = mcp
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(created.listing.title, "ThinkPad T480");

        let search = mcp
            .search_listings(
                &claims,
                &SearchRequest {
                    query: Some("ThinkPad".to_string()),
                    category: Some(Category::Laptop),
                    condition: Some(Condition::Used),
                    sort_by: SearchSort::Relevance,
                    limit: Some(10),
                    ..SearchRequest::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(search.items.len(), 1);
        assert_eq!(search.items[0].listing_id, created.listing_id);

        let replay = mcp
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:01Z")
            .await
            .unwrap();
        assert_eq!(replay.listing_id, created.listing_id);
    }

    #[tokio::test]
    async fn mcp_delegates_open_negotiation_and_idempotency() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            audit_repo.clone(),
            outbox_repo.clone(),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let mcp = MarketplaceMcp::new(app);
        let claims = build_claims();

        let created = mcp
            .create_listing(
                &claims,
                &build_create_request(),
                "fp-create-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();

        let open = mcp
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: created.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-1".to_string(),
                },
                "fp-open-1",
                "2026-05-04T00:00:01Z",
            )
            .await
            .unwrap();

        assert_eq!(
            open.status,
            marketplace_api_contract::NegotiationStatus::Reserved
        );
        assert!(open.reservation_lease_id.is_some());
    }
}
