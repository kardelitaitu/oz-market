use crate::http::handlers::{
    begin_create_listing, begin_open_negotiation, get_listing, search_listings,
};
use crate::models::db::SellerAccountRow;
use crate::repositories::{
    AuditEventRepository, IdempotencyKeyRepository, ListingRepository, OutboxEventRepository,
    RepositoryError, RepositoryErrorKind, ReservationLeaseRepository, SellerAccountRepository,
};
use crate::services::audit_events::AuditEventService;
use crate::services::contact_reveals::ContactRevealService;
use crate::services::idempotency::IdempotencyGuard;
use crate::services::outbox_events::OutboxEventService;
use crate::services::rate_limiter::{
    global_limiter, is_new_seller, NEW_SELLER_DAILY_MAX, NEW_SELLER_HOURLY_MAX,
};
use crate::services::reservations::ReservationLeaseService;
use crate::services::search::SearchService;
use marketplace_api_contract::{
    ContactRevealResponse, CreateListingRequest, CreateListingResponse, ListingStatus,
    ListingSummary, NegotiationResponse, NegotiationStatus, OpenNegotiationRequest,
    RequestContactRevealRequest, SearchRequest, SearchResponse,
};
use marketplace_auth_core::{Claims, Role};
use serde_json::json;
use std::sync::Arc;

pub const APP_NAME: &str = "marketplace-server";
const NEGOTIATION_RESERVATION_TTL_SECONDS: i64 = 3600;

// Default quota per trust level (max active listings)
const DEFAULT_QUOTA_NEW: i32 = 5;
const DEFAULT_QUOTA_VERIFIED: i32 = 20;
const DEFAULT_QUOTA_TRUSTED: i32 = 100;
const DEFAULT_QUOTA_RESTRICTED: i32 = 0;

fn default_quota(trust_level: &str) -> i32 {
    match trust_level {
        "verified" => DEFAULT_QUOTA_VERIFIED,
        "trusted" => DEFAULT_QUOTA_TRUSTED,
        "restricted" => DEFAULT_QUOTA_RESTRICTED,
        _ => DEFAULT_QUOTA_NEW, // "new" or unknown
    }
}

pub struct MarketplaceApp<LR, IR, RR, CR> {
    listing_repository: Arc<LR>,
    search: SearchService<LR>,
    idempotency: IdempotencyGuard<IR>,
    reservations: ReservationLeaseService<RR>,
    contact_reveals: ContactRevealService<CR>,
    audit_events: AuditEventService,
    outbox_events: OutboxEventService,
    seller_accounts: Arc<dyn SellerAccountRepository>,
}

impl<LR, IR, RR, CR> MarketplaceApp<LR, IR, RR, CR>
where
    LR: ListingRepository + Send + Sync,
    IR: IdempotencyKeyRepository + Send + Sync,
    RR: ReservationLeaseRepository + Send + Sync,
    CR: crate::repositories::ContactRevealRepository + Send + Sync,
{
    pub fn new(
        listing_repository: LR,
        idempotency_repository: IR,
        reservation_repository: RR,
        contact_reveal_repository: CR,
        audit_event_repository: Arc<dyn AuditEventRepository>,
        outbox_event_repository: Arc<dyn OutboxEventRepository>,
        seller_account_repository: Arc<dyn SellerAccountRepository>,
    ) -> Self {
        let listing_repository = Arc::new(listing_repository);
        let idempotency_repository = Arc::new(idempotency_repository);
        let reservation_repository = Arc::new(reservation_repository);
        let contact_reveal_repository = Arc::new(contact_reveal_repository);
        Self {
            listing_repository: Arc::clone(&listing_repository),
            search: SearchService::new(listing_repository),
            idempotency: IdempotencyGuard::new(idempotency_repository),
            reservations: ReservationLeaseService::new(reservation_repository),
            contact_reveals: ContactRevealService::new(contact_reveal_repository),
            audit_events: AuditEventService::new(audit_event_repository),
            outbox_events: OutboxEventService::new(outbox_event_repository),
            seller_accounts: seller_account_repository,
        }
    }

    pub async fn search_listings(
        &self,
        claims: Option<&Claims>,
        request: &SearchRequest,
    ) -> Result<SearchResponse, crate::http::handlers::HandlerError> {
        search_listings(&self.search, claims, request).await
    }

    pub async fn get_listing(
        &self,
        claims: Option<&Claims>,
        listing_id: &str,
    ) -> Result<Option<ListingSummary>, crate::http::handlers::HandlerError> {
        get_listing(&self.search, claims, listing_id).await
    }

    pub async fn begin_create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<
        crate::services::idempotency::IdempotencyDecision,
        crate::http::handlers::HandlerError,
    > {
        begin_create_listing(
            &self.idempotency,
            claims,
            request,
            request_fingerprint,
            now_rfc3339,
        )
        .await
    }

    pub async fn begin_open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<
        crate::services::idempotency::IdempotencyDecision,
        crate::http::handlers::HandlerError,
    > {
        begin_open_negotiation(
            &self.idempotency,
            claims,
            request,
            request_fingerprint,
            now_rfc3339,
        )
        .await
    }

    pub async fn open_negotiation(
        &self,
        claims: &Claims,
        request: &OpenNegotiationRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<NegotiationResponse, crate::http::handlers::HandlerError> {
        let attempt = crate::services::idempotency::IdempotencyAttempt {
            actor_subject: &claims.sub,
            operation: crate::services::idempotency::IdempotencyOperation::OpenNegotiation,
            idempotency_key: &request.idempotency_key,
            request_fingerprint,
            ttl_seconds: 24 * 60 * 60,
        };

        match self.idempotency.begin(&attempt, now_rfc3339).await? {
            crate::services::idempotency::IdempotencyDecision::FirstUse => {
                let listing = self
                    .search
                    .get_listing(Some(claims), &request.listing_id)
                    .await?
                    .ok_or_else(|| {
                        RepositoryError::new(RepositoryErrorKind::NotFound, "listing not found")
                    })?;

                if listing.status != marketplace_api_contract::ListingStatus::Active {
                    return Err(RepositoryError::new(
                        RepositoryErrorKind::Conflict,
                        "listing is not active",
                    )
                    .into());
                }

                let reservation = match self
                    .reservations
                    .reserve(
                        &request.listing_id,
                        &format!("neg_{}", request.listing_id),
                        &request.buyer_agent_id,
                        now_rfc3339,
                        NEGOTIATION_RESERVATION_TTL_SECONDS,
                    )
                    .await
                {
                    Ok(reservation) => reservation,
                    Err(error) => {
                        let _ = self
                            .idempotency
                            .commit_failure(&attempt, Some(json!({ "error": error.to_string() })))
                            .await;
                        return Err(error.into());
                    }
                };

                let response = NegotiationResponse {
                    negotiation_id: format!("neg_{}", request.listing_id),
                    listing_id: request.listing_id.clone(),
                    buyer_agent_id: request.buyer_agent_id.clone(),
                    status: NegotiationStatus::Reserved,
                    offer_currency: request.offer_currency.clone(),
                    latest_offer_amount: request.offer_amount,
                    reservation_lease_id: Some(reservation.lease_id.clone()),
                    final_offer_amount: None,
                    version: 1,
                    updated_at: now_rfc3339.to_string(),
                };

                self.idempotency
                    .commit_success(&attempt, json!(response))
                    .await?;
                self.record_audit_event(
                    "negotiation",
                    &response.negotiation_id,
                    "open_negotiation",
                    claims,
                    Some(&request.idempotency_key),
                    json!({
                        "request": request,
                        "response": &response,
                        "reservation_lease_id": response.reservation_lease_id.clone(),
                    }),
                    now_rfc3339,
                )
                .await?;
                self.record_outbox_event(
                    "negotiation.opened",
                    "negotiation",
                    &response.negotiation_id,
                    json!({
                        "negotiation_id": response.negotiation_id.clone(),
                        "listing_id": response.listing_id.clone(),
                        "status": response.status,
                        "reservation_lease_id": response.reservation_lease_id.clone(),
                    }),
                    now_rfc3339,
                )
                .await?;
                Ok(response)
            }
            crate::services::idempotency::IdempotencyDecision::ReplayAccepted {
                response_payload,
            } => {
                let payload = response_payload.ok_or_else(|| {
                    crate::services::idempotency::IdempotencyError::new(
                        crate::services::idempotency::IdempotencyErrorKind::Conflict,
                        "replayed open negotiation missing stored response payload",
                    )
                })?;
                serde_json::from_value::<NegotiationResponse>(payload)
                    .map_err(|error| {
                        crate::services::idempotency::IdempotencyError::new(
                            crate::services::idempotency::IdempotencyErrorKind::Storage,
                            error.to_string(),
                        )
                    })
                    .map_err(crate::http::handlers::HandlerError::from)
            }
            crate::services::idempotency::IdempotencyDecision::InFlight => {
                Err(crate::services::idempotency::IdempotencyError::new(
                    crate::services::idempotency::IdempotencyErrorKind::Conflict,
                    "idempotency key is still in flight",
                )
                .into())
            }
        }
    }

    pub async fn get_negotiation_status(
        &self,
        claims: &Claims,
        negotiation_id: &str,
    ) -> Result<NegotiationResponse, crate::http::handlers::HandlerError> {
        let listing_id = negotiation_id
            .strip_prefix("neg_")
            .unwrap_or(negotiation_id);
        let listing = self
            .search
            .get_listing(Some(claims), listing_id)
            .await?
            .ok_or_else(|| {
                RepositoryError::new(RepositoryErrorKind::NotFound, "listing not found")
            })?;
        let reservation = self.reservations.get_active_by_listing(listing_id).await?;
        let status = if reservation.is_some() {
            NegotiationStatus::Reserved
        } else {
            NegotiationStatus::Open
        };
        Ok(NegotiationResponse {
            negotiation_id: negotiation_id.to_string(),
            listing_id: listing.listing_id,
            buyer_agent_id: claims.buyer_agent_id.clone().unwrap_or_default(),
            status,
            offer_currency: listing.listing.price.currency,
            latest_offer_amount: listing.listing.price.amount,
            reservation_lease_id: reservation.map(|lease| lease.lease_id),
            final_offer_amount: None,
            version: listing.version,
            updated_at: now_marker(),
        })
    }

    pub async fn create_listing(
        &self,
        claims: &Claims,
        request: &CreateListingRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<CreateListingResponse, crate::http::handlers::HandlerError> {
        crate::services::authz::authorize_create_listing(claims, &request.listing.owner_id)?;

        // Check quota before proceeding
        let owner_id = &request.listing.owner_id;
        let seller_account = self
            .seller_accounts
            .get_by_owner_id(owner_id)
            .await
            .map_err(crate::http::handlers::HandlerError::from)?;

        if let Some(account) = &seller_account {
            let effective_quota = account
                .quota_override
                .unwrap_or_else(|| default_quota(&account.trust_level));
            if account.listings_created >= effective_quota {
                return Err(crate::http::handlers::HandlerError::QuotaExceeded {
                    message: format!(
                        "seller has exceeded quota: {} listings created (quota: {})",
                        account.listings_created, effective_quota
                    ),
                });
            }

            // Time-windowed quotas for new sellers
            if is_new_seller(&account.trust_level) {
                let daily_key = format!("daily:seller:{}", account.seller_account_id);
                let hourly_key = format!("hourly:seller:{}", account.seller_account_id);
                if !global_limiter().check(&daily_key, NEW_SELLER_DAILY_MAX as usize, 86400) {
                    return Err(crate::http::handlers::HandlerError::QuotaExceeded {
                        message: format!(
                            "new seller daily limit: {} listings/day",
                            NEW_SELLER_DAILY_MAX
                        ),
                    });
                }
                if !global_limiter().check(&hourly_key, NEW_SELLER_HOURLY_MAX as usize, 3600) {
                    return Err(crate::http::handlers::HandlerError::QuotaExceeded {
                        message: format!(
                            "new seller hourly limit: {} listing/hour",
                            NEW_SELLER_HOURLY_MAX
                        ),
                    });
                }
            }
        }

        let attempt = crate::services::idempotency::IdempotencyAttempt {
            actor_subject: &claims.sub,
            operation: crate::services::idempotency::IdempotencyOperation::CreateListing,
            idempotency_key: &request.idempotency_key,
            request_fingerprint,
            ttl_seconds: 24 * 60 * 60,
        };

        match self.idempotency.begin(&attempt, now_rfc3339).await? {
            crate::services::idempotency::IdempotencyDecision::FirstUse => {
                let response = self
                    .listing_repository
                    .as_ref()
                    .insert_listing(request)
                    .await?;

                // Increment the seller's listing count
                if let Some(account) = self
                    .seller_accounts
                    .get_by_owner_id(owner_id)
                    .await
                    .map_err(crate::http::handlers::HandlerError::from)?
                {
                    self.seller_accounts
                        .increment_listings_created(&account.seller_account_id)
                        .await
                        .map_err(crate::http::handlers::HandlerError::from)?;
                }

                self.idempotency
                    .commit_success(&attempt, json!(response))
                    .await?;
                self.record_audit_event(
                    "listing",
                    &response.listing_id,
                    "create_listing",
                    claims,
                    Some(&request.idempotency_key),
                    json!({
                        "request": request,
                        "response": &response,
                    }),
                    now_rfc3339,
                )
                .await?;
                self.record_outbox_event(
                    "listing.created",
                    "listing",
                    &response.listing_id,
                    json!({
                        "listing_id": response.listing_id.clone(),
                        "owner_id": response.listing.owner_id.clone(),
                        "status": response.status,
                    }),
                    now_rfc3339,
                )
                .await?;
                Ok(response)
            }
            crate::services::idempotency::IdempotencyDecision::ReplayAccepted {
                response_payload,
            } => {
                let payload = response_payload.ok_or_else(|| {
                    crate::services::idempotency::IdempotencyError::new(
                        crate::services::idempotency::IdempotencyErrorKind::Conflict,
                        "replayed create listing missing stored response payload",
                    )
                })?;
                serde_json::from_value::<CreateListingResponse>(payload)
                    .map_err(|error| {
                        crate::services::idempotency::IdempotencyError::new(
                            crate::services::idempotency::IdempotencyErrorKind::Storage,
                            error.to_string(),
                        )
                    })
                    .map_err(crate::http::handlers::HandlerError::from)
            }
            crate::services::idempotency::IdempotencyDecision::InFlight => {
                Err(crate::services::idempotency::IdempotencyError::new(
                    crate::services::idempotency::IdempotencyErrorKind::Conflict,
                    "idempotency key is still in flight",
                )
                .into())
            }
        }
    }

    pub async fn request_contact_reveal(
        &self,
        claims: &Claims,
        negotiation_id: &str,
        request: &RequestContactRevealRequest,
        request_fingerprint: &str,
        now_rfc3339: &str,
    ) -> Result<ContactRevealResponse, crate::http::handlers::HandlerError> {
        let seller_account_id = claims.seller_account_id.as_deref().unwrap_or(&claims.sub);
        let buyer_agent_id = claims.buyer_agent_id.as_deref().unwrap_or(&claims.sub);
        crate::services::authz::authorize_request_contact_reveal(
            claims,
            seller_account_id,
            buyer_agent_id,
        )?;
        let listing_id = negotiation_id
            .strip_prefix("neg_")
            .unwrap_or(negotiation_id);
        let reservation = self.reservations.get_active_by_listing(listing_id).await?;
        if reservation.is_none() {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Conflict,
                "reservation required before contact reveal",
            )
            .into());
        }

        let attempt = crate::services::idempotency::IdempotencyAttempt {
            actor_subject: &claims.sub,
            operation: crate::services::idempotency::IdempotencyOperation::RequestContactReveal,
            idempotency_key: &request.idempotency_key,
            request_fingerprint,
            ttl_seconds: 24 * 60 * 60,
        };

        match self.idempotency.begin(&attempt, now_rfc3339).await? {
            crate::services::idempotency::IdempotencyDecision::FirstUse => {
                let response = self
                    .contact_reveals
                    .create_request(negotiation_id, request, buyer_agent_id, now_rfc3339)
                    .await?;
                self.idempotency
                    .commit_success(&attempt, json!(response))
                    .await?;
                self.record_audit_event(
                    "contact_reveal",
                    &response.reveal_id,
                    "request_contact_reveal",
                    claims,
                    Some(&request.idempotency_key),
                    json!({
                        "request": request,
                        "response": &response,
                    }),
                    now_rfc3339,
                )
                .await?;
                self.record_outbox_event(
                    "contact_reveal.requested",
                    "contact_reveal",
                    &response.reveal_id,
                    json!({
                        "reveal_id": response.reveal_id.clone(),
                        "negotiation_id": response.negotiation_id.clone(),
                        "status": response.reveal_status,
                    }),
                    now_rfc3339,
                )
                .await?;
                Ok(response)
            }
            crate::services::idempotency::IdempotencyDecision::ReplayAccepted {
                response_payload,
            } => {
                let payload = response_payload.ok_or_else(|| {
                    crate::services::idempotency::IdempotencyError::new(
                        crate::services::idempotency::IdempotencyErrorKind::Conflict,
                        "replayed contact reveal missing stored response payload",
                    )
                })?;
                serde_json::from_value::<ContactRevealResponse>(payload)
                    .map_err(|error| {
                        crate::services::idempotency::IdempotencyError::new(
                            crate::services::idempotency::IdempotencyErrorKind::Storage,
                            error.to_string(),
                        )
                    })
                    .map_err(crate::http::handlers::HandlerError::from)
            }
            crate::services::idempotency::IdempotencyDecision::InFlight => {
                Err(crate::services::idempotency::IdempotencyError::new(
                    crate::services::idempotency::IdempotencyErrorKind::Conflict,
                    "idempotency key is still in flight",
                )
                .into())
            }
        }
    }

    pub async fn approve_contact_reveal(
        &self,
        claims: &Claims,
        reveal_id: &str,
    ) -> Result<ContactRevealResponse, crate::http::handlers::HandlerError> {
        let seller_account_id = claims.seller_account_id.as_deref().unwrap_or(&claims.sub);
        crate::services::authz::authorize_approve_contact_reveal(claims, seller_account_id)?;
        let now = now_marker();
        let response = self
            .contact_reveals
            .approve_request(reveal_id, &now)
            .await?;
        self.record_audit_event(
            "contact_reveal",
            &response.reveal_id,
            "approve_contact_reveal",
            claims,
            None,
            json!({
                "response": &response,
            }),
            &now,
        )
        .await?;
        self.record_outbox_event(
            "contact_reveal.approved",
            "contact_reveal",
            &response.reveal_id,
            json!({
                "reveal_id": response.reveal_id.clone(),
                "negotiation_id": response.negotiation_id.clone(),
                "status": response.reveal_status,
                "revealed_phone_reference": response.revealed_phone_reference.clone(),
            }),
            &now,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_contact_reveal(
        &self,
        reveal_id: &str,
    ) -> Result<Option<ContactRevealResponse>, crate::http::handlers::HandlerError> {
        Ok(self.contact_reveals.get_by_reveal_id(reveal_id).await?)
    }

    pub async fn release_reservation(
        &self,
        claims: &Claims,
        listing_id: &str,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<Option<crate::models::db::ReservationLeaseRow>, crate::http::handlers::HandlerError>
    {
        let lease = self.reservations.get_active_by_listing(listing_id).await?;
        let Some(lease) = lease else {
            return Ok(None);
        };
        let released = self
            .reservations
            .release(&lease.lease_id, now_rfc3339)
            .await?;
        self.record_audit_event(
            "reservation_lease",
            listing_id,
            "release_reservation",
            claims,
            None,
            json!({
                "reason": reason,
                "before": reservation_lease_snapshot(&lease),
                "after": released.as_ref().map(reservation_lease_snapshot),
            }),
            now_rfc3339,
        )
        .await?;
        Ok(released)
    }

    pub async fn archive_listing(
        &self,
        claims: &Claims,
        listing_id: &str,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<Option<ListingSummary>, crate::http::handlers::HandlerError> {
        if !claims.has_role(Role::Admin) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::PermissionDenied,
                "archive requires admin role",
            )
            .into());
        }
        let before = self.get_listing(Some(claims), listing_id).await?;
        let updated = self
            .listing_repository
            .as_ref()
            .update_listing_status(listing_id, ListingStatus::Archived)
            .await?;
        if let Some(ref summary) = updated {
            self.record_audit_event(
                "listing",
                listing_id,
                "archive_listing",
                claims,
                None,
                json!({
                    "reason": reason,
                    "before_status": before.as_ref().map(|b| format!("{:?}", b.status)),
                    "after_status": format!("{:?}", summary.status),
                }),
                now_rfc3339,
            )
            .await?;
            self.record_outbox_event(
                "listing.archived",
                "listing",
                listing_id,
                json!({
                    "listing_id": listing_id,
                    "status": summary.status,
                }),
                now_rfc3339,
            )
            .await?;
        }
        Ok(updated)
    }

    pub async fn set_seller_trust_level(
        &self,
        claims: &Claims,
        seller_account_id: &str,
        trust_level: &str,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<Option<SellerAccountRow>, crate::http::handlers::HandlerError> {
        if !claims.has_role(Role::Admin) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::PermissionDenied,
                "trust level change requires admin role",
            )
            .into());
        }
        let updated = self
            .seller_accounts
            .update_trust_level(seller_account_id, trust_level)
            .await?;
        if let Some(ref _account) = updated {
            self.record_audit_event(
                "seller_account",
                seller_account_id,
                "set_trust_level",
                claims,
                None,
                json!({
                    "reason": reason,
                    "trust_level": trust_level,
                }),
                now_rfc3339,
            )
            .await?;
        }
        Ok(updated)
    }

    pub async fn set_seller_quota_override(
        &self,
        claims: &Claims,
        seller_account_id: &str,
        quota_override: Option<i32>,
        reason: &str,
        now_rfc3339: &str,
    ) -> Result<Option<SellerAccountRow>, crate::http::handlers::HandlerError> {
        if !claims.has_role(Role::Admin) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::PermissionDenied,
                "quota override requires admin role",
            )
            .into());
        }
        let updated = self
            .seller_accounts
            .update_quota_override(seller_account_id, quota_override)
            .await?;
        if let Some(ref _account) = updated {
            self.record_audit_event(
                "seller_account",
                seller_account_id,
                "set_quota_override",
                claims,
                None,
                json!({
                    "reason": reason,
                    "quota_override": quota_override,
                }),
                now_rfc3339,
            )
            .await?;
        }
        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_audit_event(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: &str,
        claims: &Claims,
        idempotency_key: Option<&str>,
        payload: serde_json::Value,
        now_rfc3339: &str,
    ) -> Result<(), crate::http::handlers::HandlerError> {
        self.audit_events
            .append_event(crate::models::db::AuditEventRow {
                event_id: 0,
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                action: action.to_string(),
                actor_subject: claims.sub.clone(),
                actor_role: claims
                    .roles
                    .first()
                    .map(|role| format!("{role:?}"))
                    .unwrap_or_default(),
                scopes: claims
                    .scopes
                    .iter()
                    .map(|scope| format!("{scope:?}"))
                    .collect(),
                request_id: None,
                idempotency_key: idempotency_key.map(|value| value.to_string()),
                payload,
                created_at: now_rfc3339.to_string(),
            })
            .await?;
        Ok(())
    }

    async fn record_outbox_event(
        &self,
        topic: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
        now_rfc3339: &str,
    ) -> Result<(), crate::http::handlers::HandlerError> {
        self.outbox_events
            .append_event(crate::models::db::OutboxEventRow {
                event_id: 0,
                topic: topic.to_string(),
                aggregate_type: aggregate_type.to_string(),
                aggregate_id: aggregate_id.to_string(),
                payload,
                available_at: now_rfc3339.to_string(),
                published_at: None,
                attempt_count: 0,
                created_at: now_rfc3339.to_string(),
            })
            .await?;
        Ok(())
    }
}

fn now_marker() -> String {
    // Use chrono::Utc::now() to get current time in RFC3339 format
    let now = chrono::Utc::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn reservation_lease_snapshot(lease: &crate::models::db::ReservationLeaseRow) -> serde_json::Value {
    json!({
        "lease_id": lease.lease_id.clone(),
        "negotiation_id": lease.negotiation_id.clone(),
        "listing_id": lease.listing_id.clone(),
        "reserved_by": lease.reserved_by.clone(),
        "status": lease.status.clone(),
        "expires_at": lease.expires_at.clone(),
        "created_at": lease.created_at.clone(),
        "updated_at": lease.updated_at.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::handlers::HandlerError;
    use crate::repositories::audit_events::InMemoryAuditEventRepository;
    use crate::repositories::contact_reveals::InMemoryContactRevealRepository;
    use crate::repositories::listings::InMemoryListingRepository;
    use crate::repositories::outbox_events::InMemoryOutboxEventRepository;
    use crate::repositories::reservations::InMemoryReservationLeaseRepository;
    use crate::repositories::seller_accounts::InMemorySellerAccountRepository;
    use crate::repositories::{ListingRepository, RepositoryError, RepositoryErrorKind};
    use crate::services::idempotency::InMemoryIdempotencyRepository;
    use marketplace_api_contract::{
        Category, Condition, ContactRevealStatus, CreateListingRequest, ListingLocation,
        ListingPayload, ListingStatus, ListingSummary, OpenNegotiationRequest, Price,
        RequestContactRevealRequest, SearchRequest, SearchResponse, SearchSort,
    };
    use marketplace_auth_core::Claims;
    use serde_json::json;

    struct SoldListingRepository;

    #[async_trait::async_trait]
    impl ListingRepository for SoldListingRepository {
        async fn insert_listing(
            &self,
            _request: &CreateListingRequest,
        ) -> Result<CreateListingResponse, RepositoryError> {
            Err(RepositoryError::new(
                RepositoryErrorKind::Conflict,
                "sold listing repository is read-only in tests",
            ))
        }

        async fn get_listing(
            &self,
            listing_id: &str,
        ) -> Result<Option<ListingSummary>, RepositoryError> {
            let request = create_request();
            Ok(Some(ListingSummary {
                listing_id: listing_id.to_string(),
                status: ListingStatus::Sold,
                version: 9,
                listing: request.listing,
                // Seller fields (read-only, None for tests)
                seller_name: None,
                seller_rating: None,
                seller_verified: None,
            }))
        }

        async fn search_listings(
            &self,
            request: &SearchRequest,
        ) -> Result<SearchResponse, RepositoryError> {
            Ok(SearchResponse {
                items: vec![],
                applied_sort_by: request.sort_by,
                next_cursor: None,
            })
        }

        async fn update_listing_status(
            &self,
            _listing_id: &str,
            _status: ListingStatus,
        ) -> Result<Option<ListingSummary>, RepositoryError> {
            Ok(None)
        }
    }

    fn claims() -> Claims {
        crate::test_support::seller_claims()
    }

    fn admin_claims() -> Claims {
        crate::test_support::admin_claims()
    }

    fn create_request() -> CreateListingRequest {
        CreateListingRequest {
            idempotency_key: "idem-create-1".to_string(),
            listing: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "seller-1".to_string(),
                listing_type: marketplace_api_contract::ListingType::Product,
                category: Some(Category::Laptop),
                title: "ThinkPad T480".to_string(),
                condition: Some(Condition::Used),
                // NEW: Phase 2 fields
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
                price: Price {
                    currency: "USD".to_string(),
                    amount: 450.0,
                },
                location: ListingLocation {
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
    async fn shared_app_reserves_on_open_and_reveals_contact() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo.clone(),
            outbox_repo.clone(),
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let listing = app
            .create_listing(
                &claims,
                &create_request(),
                "fp-create-1",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        assert_eq!(audit_repo.events().len(), 1);
        assert_eq!(outbox_repo.events().len(), 1);

        let open = app
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: listing.listing_id.clone(),
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
        assert_eq!(open.status, NegotiationStatus::Reserved);
        assert!(open.reservation_lease_id.is_some());
        assert_eq!(audit_repo.events().len(), 2);
        assert_eq!(outbox_repo.events().len(), 2);

        let second_open = app
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: listing.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 441.0,
                    idempotency_key: "idem-open-2".to_string(),
                },
                "fp-open-2",
                "2026-05-04T00:00:03Z",
            )
            .await;
        assert!(matches!(
            second_open,
            Err(crate::http::handlers::HandlerError::Repository(
                RepositoryError {
                    kind: RepositoryErrorKind::Conflict,
                    ..
                }
            ))
        ));

        let reveal = app
            .request_contact_reveal(
                &claims,
                &open.negotiation_id,
                &RequestContactRevealRequest {
                    idempotency_key: "idem-reveal-1".to_string(),
                },
                "fp-reveal-1",
                "2026-05-04T00:00:05Z",
            )
            .await
            .unwrap();
        assert_eq!(reveal.reveal_status, ContactRevealStatus::Pending);
        assert_eq!(audit_repo.events().len(), 3);
        assert_eq!(outbox_repo.events().len(), 3);

        let approved = app
            .approve_contact_reveal(&claims, &reveal.reveal_id)
            .await
            .unwrap();
        assert_eq!(approved.reveal_status, ContactRevealStatus::Approved);
        assert_eq!(audit_repo.events().len(), 4);
        assert_eq!(outbox_repo.events().len(), 4);
    }

    #[tokio::test]
    async fn shared_app_rejects_open_negotiation_for_sold_listing() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            SoldListingRepository,
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let error = app
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: "lst-sold".to_string(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-sold".to_string(),
                },
                "fp-open-sold",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            crate::http::handlers::HandlerError::Repository(RepositoryError {
                kind: RepositoryErrorKind::Conflict,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn shared_app_rejects_contact_reveal_without_reservation() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let error = app
            .request_contact_reveal(
                &claims,
                "neg_lst_404",
                &RequestContactRevealRequest {
                    idempotency_key: "idem-reveal-missing-reservation".to_string(),
                },
                "fp-reveal-missing-reservation",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            crate::http::handlers::HandlerError::Repository(RepositoryError {
                kind: RepositoryErrorKind::Conflict,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn shared_app_audits_internal_reservation_release() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo.clone(),
            outbox_repo,
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let buyer = claims();
        let admin = admin_claims();

        let created = app
            .create_listing(
                &buyer,
                &create_request(),
                "fp-create-admin",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        let opened = app
            .open_negotiation(
                &buyer,
                &OpenNegotiationRequest {
                    listing_id: created.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-admin".to_string(),
                },
                "fp-open-admin",
                "2026-05-04T00:00:01Z",
            )
            .await
            .unwrap();

        let released = app
            .release_reservation(
                &admin,
                &created.listing_id,
                "admin cleanup",
                "2026-05-04T00:01:00Z",
            )
            .await
            .unwrap();
        assert!(released.is_some());
        assert_eq!(opened.status, NegotiationStatus::Reserved);
        assert_eq!(audit_repo.events().len(), 3);
        let events = audit_repo.events();
        let last = events.last().unwrap();
        assert_eq!(last.action, "release_reservation");
        assert!(last.payload.to_string().contains("admin cleanup"));
    }

    #[tokio::test]
    async fn shared_app_rejects_internal_archive_for_support_reviewer() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = crate::test_support::support_claims();

        let error = app
            .archive_listing(&claims, "lst_000001", "cleanup", "2026-05-04T00:00:00Z")
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            crate::http::handlers::HandlerError::Repository(RepositoryError {
                kind: RepositoryErrorKind::PermissionDenied,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn shared_app_allows_internal_archive_for_admin() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let seller = crate::test_support::seller_claims();
        let admin = crate::test_support::admin_claims();

        let created = app
            .create_listing(
                &seller,
                &create_request(),
                "fp-create-admin-archive",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        let archived = app
            .archive_listing(
                &admin,
                &created.listing_id,
                "cleanup",
                "2026-05-04T00:01:00Z",
            )
            .await
            .unwrap();

        assert!(archived.is_some());
        assert_eq!(archived.unwrap().status, ListingStatus::Archived);
    }

    #[tokio::test]
    async fn shared_app_searches_and_reads_listings() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let request = create_request();

        let created = app
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(created.listing.title, "ThinkPad T480");

        let response = app
            .search_listings(
                Some(&claims),
                &SearchRequest {
                    query: Some("ThinkPad".to_string()),
                    category: Some(Category::Laptop),
                    condition: Some(Condition::Used),
                    // NEW: Phase 2 fields
                    listing_type: Some(marketplace_api_contract::ListingType::Product),
                    service_type: None,
                    property_transaction_type: None,
                    property_sub_type: None,
                    sort_by: SearchSort::Relevance,
                    limit: Some(10),
                    ..SearchRequest::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].listing_id, created.listing_id);
        let get = app
            .get_listing(Some(&claims), &created.listing_id)
            .await
            .unwrap();
        assert!(get.is_some());
    }

    #[tokio::test]
    async fn shared_app_reuses_create_idempotency_key() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo.clone(),
            outbox_repo.clone(),
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let request = create_request();

        let first = app
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(first.listing.title, "ThinkPad T480");

        let replay = app
            .create_listing(&claims, &request, "fp-create-1", "2026-05-04T00:00:01Z")
            .await
            .unwrap();
        assert_eq!(replay.listing_id, first.listing_id);
        assert_eq!(audit_repo.events().len(), 1);
        assert_eq!(outbox_repo.events().len(), 1);
    }

    #[tokio::test]
    async fn shared_app_reuses_open_negotiation_idempotency_key() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo.clone(),
            outbox_repo.clone(),
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let listing = app
            .create_listing(
                &claims,
                &create_request(),
                "fp-create-open-replay",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();

        let request = OpenNegotiationRequest {
            listing_id: listing.listing_id.clone(),
            buyer_agent_id: "buyer-1".to_string(),
            offer_currency: "USD".to_string(),
            offer_amount: 440.0,
            idempotency_key: "idem-open-replay".to_string(),
        };
        let first = app
            .open_negotiation(&claims, &request, "fp-open-replay", "2026-05-04T00:00:01Z")
            .await
            .unwrap();
        let replay = app
            .open_negotiation(&claims, &request, "fp-open-replay", "2026-05-04T00:00:02Z")
            .await
            .unwrap();

        assert_eq!(replay.negotiation_id, first.negotiation_id);
        assert_eq!(audit_repo.events().len(), 2);
        assert_eq!(outbox_repo.events().len(), 2);
    }

    #[tokio::test]
    async fn shared_app_reuses_contact_reveal_idempotency_key() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo.clone(),
            outbox_repo.clone(),
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        );
        let claims = claims();
        let listing = app
            .create_listing(
                &claims,
                &create_request(),
                "fp-create-reveal-replay",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        let open = app
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: listing.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-reveal-replay".to_string(),
                },
                "fp-open-reveal-replay",
                "2026-05-04T00:00:01Z",
            )
            .await
            .unwrap();

        let request = RequestContactRevealRequest {
            idempotency_key: "idem-reveal-replay".to_string(),
        };
        let first = app
            .request_contact_reveal(
                &claims,
                &open.negotiation_id,
                &request,
                "fp-reveal-replay",
                "2026-05-04T00:00:02Z",
            )
            .await
            .unwrap();
        let replay = app
            .request_contact_reveal(
                &claims,
                &open.negotiation_id,
                &request,
                "fp-reveal-replay",
                "2026-05-04T00:00:03Z",
            )
            .await
            .unwrap();

        assert_eq!(replay.reveal_id, first.reveal_id);
        assert_eq!(audit_repo.events().len(), 3);
        assert_eq!(outbox_repo.events().len(), 3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shared_app_blocks_concurrent_open_negotiations_on_same_listing() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = Arc::new(MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        ));
        let claims = claims();
        let listing = app
            .create_listing(
                &claims,
                &create_request(),
                "fp-create-race",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();

        let first_request = OpenNegotiationRequest {
            listing_id: listing.listing_id.clone(),
            buyer_agent_id: "buyer-1".to_string(),
            offer_currency: "USD".to_string(),
            offer_amount: 440.0,
            idempotency_key: "idem-open-race-1".to_string(),
        };
        let second_request = OpenNegotiationRequest {
            listing_id: listing.listing_id.clone(),
            buyer_agent_id: "buyer-2".to_string(),
            offer_currency: "USD".to_string(),
            offer_amount: 441.0,
            idempotency_key: "idem-open-race-2".to_string(),
        };

        let first_app = Arc::clone(&app);
        let second_app = Arc::clone(&app);
        let claims_one = claims.clone();
        let claims_two = claims.clone();
        let first = tokio::spawn(async move {
            first_app
                .open_negotiation(
                    &claims_one,
                    &first_request,
                    "fp-open-race-1",
                    "2026-05-04T00:00:01Z",
                )
                .await
        });
        let second = tokio::spawn(async move {
            second_app
                .open_negotiation(
                    &claims_two,
                    &second_request,
                    "fp-open-race-2",
                    "2026-05-04T00:00:01Z",
                )
                .await
        });

        let (first, second) = tokio::join!(first, second);
        let first = first.unwrap();
        let second = second.unwrap();
        let results = [first, second];
        let conflicts = results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(HandlerError::Repository(RepositoryError {
                        kind: RepositoryErrorKind::Conflict,
                        ..
                    }))
                )
            })
            .count();
        let successes = results.iter().filter(|result| result.is_ok()).count();
        assert_eq!(successes, 1);
        assert_eq!(conflicts, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shared_app_blocks_concurrent_contact_approvals() {
        let audit_repo = Arc::new(InMemoryAuditEventRepository::new());
        let outbox_repo = Arc::new(InMemoryOutboxEventRepository::new());
        let app = Arc::new(MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            audit_repo,
            outbox_repo,
            std::sync::Arc::new(InMemorySellerAccountRepository::new()),
        ));
        let claims = claims();
        let listing = app
            .create_listing(
                &claims,
                &create_request(),
                "fp-create-approve-race",
                "2026-05-04T00:00:00Z",
            )
            .await
            .unwrap();
        let open = app
            .open_negotiation(
                &claims,
                &OpenNegotiationRequest {
                    listing_id: listing.listing_id.clone(),
                    buyer_agent_id: "buyer-1".to_string(),
                    offer_currency: "USD".to_string(),
                    offer_amount: 440.0,
                    idempotency_key: "idem-open-approve-race".to_string(),
                },
                "fp-open-approve-race",
                "2026-05-04T00:00:01Z",
            )
            .await
            .unwrap();
        let reveal = app
            .request_contact_reveal(
                &claims,
                &open.negotiation_id,
                &RequestContactRevealRequest {
                    idempotency_key: "idem-reveal-approve-race".to_string(),
                },
                "fp-reveal-approve-race",
                "2026-05-04T00:00:02Z",
            )
            .await
            .unwrap();

        let first_app = Arc::clone(&app);
        let second_app = Arc::clone(&app);
        let claims_one = claims.clone();
        let claims_two = claims.clone();
        let reveal_id_one = reveal.reveal_id.clone();
        let reveal_id_two = reveal.reveal_id.clone();
        let first = tokio::spawn(async move {
            first_app
                .approve_contact_reveal(&claims_one, &reveal_id_one)
                .await
        });
        let second = tokio::spawn(async move {
            second_app
                .approve_contact_reveal(&claims_two, &reveal_id_two)
                .await
        });

        let (first, second) = tokio::join!(first, second);
        let first = first.unwrap();
        let second = second.unwrap();
        let results = [first, second];
        let conflicts = results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(HandlerError::Repository(RepositoryError {
                        kind: RepositoryErrorKind::Conflict,
                        ..
                    }))
                )
            })
            .count();
        let successes = results.iter().filter(|result| result.is_ok()).count();
        assert_eq!(successes, 1);
        assert_eq!(conflicts, 1);
    }

    // -----------------------------------------------------------------------
    // Priority 4 — Service orchestration tests using MockListingRepository
    // -----------------------------------------------------------------------

    use crate::services::search::SearchError;
    use crate::test_support::MockListingRepository;

    #[tokio::test]
    async fn mock_storage_error_from_search_propagates() {
        let mock = MockListingRepository::new();
        mock.fail_all();
        let app = build_app_with_listing_repo(mock);

        let result = app
            .search_listings(Some(&claims()), &SearchRequest::default())
            .await;
        assert!(matches!(
            result,
            Err(HandlerError::Search(SearchError::Storage(
                RepositoryError {
                    kind: RepositoryErrorKind::Storage,
                    ..
                }
            )))
        ));
    }

    #[tokio::test]
    async fn mock_storage_error_from_get_propagates() {
        let mock = MockListingRepository::new();
        mock.fail_all();
        let app = build_app_with_listing_repo(mock);

        let result = app.get_listing(Some(&claims()), "lst_1").await;
        assert!(matches!(
            result,
            Err(HandlerError::Search(SearchError::Storage(
                RepositoryError {
                    kind: RepositoryErrorKind::Storage,
                    ..
                }
            )))
        ));
    }

    #[tokio::test]
    async fn mock_not_found_from_get_returns_none() {
        let mock = MockListingRepository::new();
        mock.get_result.lock().unwrap().replace(Ok(None));
        let app = build_app_with_listing_repo(mock);

        let result = app
            .get_listing(Some(&claims()), "lst_missing")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    fn build_app_with_listing_repo(
        listing_repo: MockListingRepository,
    ) -> MarketplaceApp<
        MockListingRepository,
        InMemoryIdempotencyRepository,
        InMemoryReservationLeaseRepository,
        InMemoryContactRevealRepository,
    > {
        MarketplaceApp::new(
            listing_repo,
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        )
    }
}
