use marketplace_api_contract::{
    ContactRevealResponse, CreateListingRequest, CreateListingResponse, ListingSummary,
    NegotiationResponse, OpenNegotiationRequest, RequestContactRevealRequest, SearchRequest,
    SearchResponse,
};
use marketplace_auth_core::Claims;

pub struct MarketplaceMcp<App> {
    app: App,
}

impl<App> MarketplaceMcp<App> {
    pub fn new(app: App) -> Self {
        Self { app }
    }
}

impl<LR, IR, RR, CR> MarketplaceMcp<marketplace_server::app::MarketplaceApp<LR, IR, RR, CR>>
where
    LR: marketplace_server::repositories::ListingRepository + Send + Sync,
    IR: marketplace_server::repositories::IdempotencyKeyRepository + Send + Sync,
    RR: marketplace_server::repositories::ReservationLeaseRepository + Send + Sync,
    CR: marketplace_server::repositories::ContactRevealRepository + Send + Sync,
{
    pub async fn search_listings(
        &self,
        claims: &Claims,
        request: &SearchRequest,
    ) -> Result<SearchResponse, marketplace_server::http::handlers::HandlerError> {
        self.app.search_listings(claims, request).await
    }

    pub async fn get_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, marketplace_server::http::handlers::HandlerError> {
        self.app.get_listing(claims, listing_id).await
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
        self.app.get_negotiation_status(claims, negotiation_id).await
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
    ) -> Result<Option<ContactRevealResponse>, marketplace_server::http::handlers::HandlerError> {
        self.app.get_contact_reveal(reveal_id).await
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::{
        Category, Condition, CreateListingRequest, ListingLocation, ListingPayload,
        OpenNegotiationRequest, Price, RequestContactRevealRequest, SearchRequest, SearchSort,
    };
    use marketplace_auth_core::{Claims, Role, Scope};
    use std::sync::Arc;

    fn claims() -> Claims {
        Claims {
            sub: "sub-1".to_string(),
            roles: vec![
                Role::SellerListingWriter,
                Role::BuyerNegotiator,
                Role::SellerContactRevealApprover,
            ],
            scopes: vec![
                Scope::ListingRead,
                Scope::ListingSearch,
                Scope::ListingCreate,
                Scope::NegotiationCreate,
                Scope::NegotiationRead,
                Scope::NegotiationRevealRequest,
                Scope::RevealApprove,
            ],
            seller_account_id: Some("seller-1".to_string()),
            buyer_agent_id: Some("buyer-1".to_string()),
            hardware_id: None,
            exp: None,
        }
    }

    fn create_request() -> CreateListingRequest {
        CreateListingRequest {
            idempotency_key: "idem-create-1".to_string(),
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "seller-1".to_string(),
                category: Category::Laptop,
                product_name: "ThinkPad T480".to_string(),
                condition: Condition::Used,
                price: Price {
                    currency: "USD".to_string(),
                    amount: 450.0,
                },
                location: ListingLocation {
                    country_code: "JP".to_string(),
                    country_name: "Japan".to_string(),
                    city: "Osaka".to_string(),
                },
                picture_urls: vec!["https://example.com/item.jpg".to_string()],
                description: "Good battery health".to_string(),
                attributes: None,
            },
        }
    }

    #[tokio::test]
    async fn mcp_delegates_search_and_idempotency_to_shared_app() {
        let listing_repo =
            marketplace_server::repositories::listings::InMemoryListingRepository::new();
        let idempotency_repo =
            marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new();
        let app = marketplace_server::app::MarketplaceApp::new(
            std::sync::Arc::new(
                marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository::new(),
            ),
            listing_repo,
            idempotency_repo,
            marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository::new(
            ),
            marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository::new(
            ),
            Arc::new(
                marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
            ),
            Arc::new(
                marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(
                ),
            ),
        );
        let mcp = MarketplaceMcp::new(app);
        let claims = claims();
        let request = create_request();

        let created = mcp
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(created.listing.product_name, "ThinkPad T480");

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

        let open = mcp
            .begin_open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: "lst_000001".to_string(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-1".to_string(),
                },
                "fp-open-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        assert!(matches!(
            open,
            marketplace_server::services::idempotency::IdempotencyDecision::FirstUse
        ));
    }

    #[tokio::test]
    async fn mcp_replays_create_listing_and_conflicts_on_fingerprint_mismatch() {
        let app = marketplace_server::app::MarketplaceApp::new(
            std::sync::Arc::new(
                marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository::new(),
            ),
            marketplace_server::repositories::listings::InMemoryListingRepository::new(),
            marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
            marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository::new(
            ),
            marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository::new(
            ),
            Arc::new(
                marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
            ),
            Arc::new(
                marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(
                ),
            ),
        );
        let mcp = MarketplaceMcp::new(app);
        let claims = claims();
        let request = create_request();

        let first = mcp
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        let replay = mcp
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:01Z")
            .await
            .unwrap();
        assert_eq!(replay.listing_id, first.listing_id);

        let conflicting = mcp
            .begin_create_listing(&claims, &request, "fp-create-2", "2026-05-04T00:00:02Z")
            .await;
        assert!(conflicting.is_err());
    }

    #[tokio::test]
    async fn mcp_consumes_state_changes_by_polling_shared_reads() {
        let app = marketplace_server::app::MarketplaceApp::new(
            std::sync::Arc::new(
                marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository::new(),
            ),
            marketplace_server::repositories::listings::InMemoryListingRepository::new(),
            marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
            marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository::new(
            ),
            marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository::new(
            ),
            Arc::new(
                marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
            ),
            Arc::new(
                marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(
                ),
            ),
        );
        let mcp = MarketplaceMcp::new(app);
        let claims = claims();
        let request = create_request();

        let created = mcp
            .create_listing(&claims, &request, "fp-create-state", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        let opened = mcp
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: created.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-state".to_string(),
                },
                "fp-open-state",
                "2026-05-04T00:00:01Z",
            )
            .await
            .unwrap();
        assert_eq!(opened.status, marketplace_api_contract::NegotiationStatus::Reserved);

        let negotiation = mcp
            .get_negotiation_status(&claims, &format!("neg_{}", created.listing_id))
            .await
            .unwrap();
        assert_eq!(
            negotiation.status,
            marketplace_api_contract::NegotiationStatus::Reserved
        );

        let reveal = mcp
            .request_contact_reveal(
                &claims,
                &negotiation.negotiation_id,
                &RequestContactRevealRequest {
                    idempotency_key: "idem-reveal-state".to_string(),
                },
                "fp-reveal-state",
                "2026-05-04T00:00:02Z",
            )
            .await
            .unwrap();
        assert_eq!(reveal.reveal_status, marketplace_api_contract::ContactRevealStatus::Pending);

        let approved = mcp
            .approve_contact_reveal(&claims, &reveal.reveal_id)
            .await
            .unwrap();
        assert_eq!(
            approved.reveal_status,
            marketplace_api_contract::ContactRevealStatus::Approved
        );

        let polled = mcp.get_contact_reveal(&reveal.reveal_id).await.unwrap();
        assert_eq!(
            polled.unwrap().reveal_status,
            marketplace_api_contract::ContactRevealStatus::Approved
        );
    }
}
