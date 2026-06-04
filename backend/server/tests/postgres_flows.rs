use marketplace_api_contract::{
    AcceptNegotiationRequest, NegotiationHistoryEntryType, NegotiationStatus,
    RejectNegotiationRequest, RequestContactRevealRequest, SubmitOfferRequest,
};
use marketplace_server::repositories::{
    negotiations::{NegotiationRepository, PostgresNegotiationRepository},
    ContactRevealRepository, PostgresContactRevealRepository, PostgresReservationLeaseRepository,
    ReservationLeaseRepository, SellerAccountRepository,
};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool, Row};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

async fn live_pool() -> Result<Option<PgPool>, Box<dyn Error + Send + Sync>> {
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
        eprintln!("skipping postgres integration tests: DATABASE_URL is not set");
        return Ok(None);
    };

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    Ok(Some(pool))
}

async fn live_bootstrapped_pool() -> Result<Option<PgPool>, Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_pool().await? else {
        return Ok(None);
    };
    marketplace_server::bootstrap::apply_schema(&pool).await?;
    Ok(Some(pool))
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}")
}

async fn seed_listing(pool: &PgPool, listing_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, $2, '1.0', 'laptop', $3, 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $4, $5, $6, 'active', 1, $7, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(listing_id)
    .bind("seller-1")
    .bind(format!("ThinkPad {listing_id}"))
    .bind(Json(json!(["https://example.com/item.jpg"])))
    .bind(format!("Good battery health for {listing_id}"))
    .bind(Json(json!({"brand": "Lenovo"})))
    .bind(format!("create-{listing_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_negotiation(
    pool: &PgPool,
    negotiation_id: &str,
    listing_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO negotiations (
            negotiation_id, listing_id, buyer_agent_id, status, offer_currency,
            latest_offer_amount, reservation_lease_id, final_offer_amount, version,
            open_idempotency_key, created_at, updated_at
        ) VALUES ($1, $2, $3, 'reserved', 'USD', 499.00, NULL, NULL, 1, $4, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(negotiation_id)
    .bind(listing_id)
    .bind("buyer-1")
    .bind(format!("open-{negotiation_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn postgres_reservation_flow_persists_and_blocks_double_sell(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_pool().await? else {
        return Ok(());
    };
    marketplace_server::bootstrap::apply_schema(&pool).await?;

    let suffix = unique_suffix();
    let listing_id = format!("lst_{suffix}");
    let negotiation_id = format!("neg_{suffix}");
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    let repo = PostgresReservationLeaseRepository::new(pool.clone());
    let lease = repo
        .reserve(
            &listing_id,
            &negotiation_id,
            "buyer-1",
            "2026-05-04T00:00:00Z",
            3600,
        )
        .await?;
    assert_eq!(lease.listing_id, listing_id);
    assert_eq!(lease.status, "active");

    let stored_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reservation_leases WHERE listing_id = $1 AND status = 'active'",
    )
    .bind(&listing_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored_count, 1);

    let second_negotiation_id = format!("neg_{}_2", suffix);
    seed_negotiation(&pool, &second_negotiation_id, &listing_id).await?;
    let second = repo
        .reserve(
            &listing_id,
            &second_negotiation_id,
            "buyer-2",
            "2026-05-04T00:01:00Z",
            3600,
        )
        .await;
    assert!(
        matches!(second, Err(err) if err.kind == marketplace_server::repositories::RepositoryErrorKind::Conflict)
    );

    Ok(())
}

#[tokio::test]
async fn postgres_contact_approval_flow_persists_and_updates_status(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_pool().await? else {
        return Ok(());
    };
    marketplace_server::bootstrap::apply_schema(&pool).await?;

    let suffix = unique_suffix();
    let listing_id = format!("lst_{suffix}");
    let negotiation_id = format!("neg_{suffix}");
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    // Seed reservation lease (create_request queries reservation_leases)
    let res_repo = PostgresReservationLeaseRepository::new(pool.clone());
    res_repo
        .reserve(
            &listing_id,
            &negotiation_id,
            "buyer-1",
            "2026-05-04T00:00:00Z",
            3600,
        )
        .await?;

    let repo = PostgresContactRevealRepository::new(pool.clone());
    let created = repo
        .create_request(
            &negotiation_id,
            &RequestContactRevealRequest {
                idempotency_key: format!("idem-{suffix}"),
            },
            "buyer-1",
            "2026-05-04T00:00:00Z",
        )
        .await?;
    assert_eq!(
        created.reveal_status,
        marketplace_api_contract::ContactRevealStatus::Pending
    );

    let approved = repo
        .approve_request(&created.reveal_id, "2026-05-04T00:01:00Z")
        .await?;
    assert_eq!(
        approved.reveal_status,
        marketplace_api_contract::ContactRevealStatus::Approved
    );

    let second = repo
        .approve_request(&created.reveal_id, "2026-05-04T00:02:00Z")
        .await;
    assert!(
        matches!(second, Err(err) if err.kind == marketplace_server::repositories::RepositoryErrorKind::Conflict)
    );

    let stored_status: String =
        sqlx::query_scalar("SELECT reveal_status FROM contact_reveals WHERE reveal_id = $1")
            .bind(&created.reveal_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored_status, "approved");

    Ok(())
}

#[tokio::test]
async fn postgres_negotiation_submit_and_accept_persist_offer_history(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_pool().await? else {
        return Ok(());
    };
    marketplace_server::bootstrap::apply_schema(&pool).await?;

    let suffix = unique_suffix();
    let listing_id = format!("lst_{suffix}");
    let negotiation_id = format!("neg_{suffix}");
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    let repo = PostgresNegotiationRepository::new(pool.clone());
    let submitted = repo
        .submit_offer(
            &negotiation_id,
            &SubmitOfferRequest {
                offer_currency: "USD".to_string(),
                offer_amount: 525.0,
                idempotency_key: format!("offer-{suffix}"),
            },
            "seller-1",
            "seller_negotiator",
            "2026-05-04T00:01:00Z",
        )
        .await?;
    assert_eq!(submitted.status, NegotiationStatus::Countered);
    assert_eq!(submitted.latest_offer_amount, 525.0);
    assert_eq!(submitted.offer_history.len(), 1);
    assert_eq!(
        submitted.offer_history[0].entry_type,
        NegotiationHistoryEntryType::Offer
    );

    let accepted = repo
        .accept_negotiation(
            &negotiation_id,
            &AcceptNegotiationRequest {
                idempotency_key: format!("accept-{suffix}"),
            },
            "seller-1",
            "seller_negotiator",
            "2026-05-04T00:02:00Z",
        )
        .await?;
    assert_eq!(accepted.status, NegotiationStatus::Closed);
    assert_eq!(accepted.final_offer_amount, Some(525.0));
    assert_eq!(accepted.offer_history.len(), 2);
    assert_eq!(
        accepted.offer_history[1].entry_type,
        NegotiationHistoryEntryType::Accept
    );

    let row = sqlx::query(
        "SELECT status, final_offer_amount, jsonb_array_length(offer_history) AS history_len
         FROM negotiations
         WHERE negotiation_id = $1",
    )
    .bind(&negotiation_id)
    .fetch_one(&pool)
    .await?;
    let stored_status: String = row.try_get("status")?;
    let stored_final_offer: Option<f64> = row.try_get("final_offer_amount")?;
    let stored_history_len: i32 = row.try_get("history_len")?;
    assert_eq!(stored_status, "closed");
    assert_eq!(stored_final_offer, Some(525.0));
    assert_eq!(stored_history_len, 2);

    Ok(())
}

#[tokio::test]
async fn postgres_negotiation_reject_persists_cancelled_state_and_history(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_pool().await? else {
        return Ok(());
    };
    marketplace_server::bootstrap::apply_schema(&pool).await?;

    let suffix = unique_suffix();
    let listing_id = format!("lst_{suffix}");
    let negotiation_id = format!("neg_{suffix}");
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    let repo = PostgresNegotiationRepository::new(pool.clone());
    let rejected = repo
        .reject_negotiation(
            &negotiation_id,
            &RejectNegotiationRequest {
                idempotency_key: format!("reject-{suffix}"),
            },
            "seller-1",
            "seller_negotiator",
            "2026-05-04T00:01:00Z",
        )
        .await?;
    assert_eq!(rejected.status, NegotiationStatus::Cancelled);
    assert_eq!(rejected.final_offer_amount, None);
    assert_eq!(rejected.offer_history.len(), 1);
    assert_eq!(
        rejected.offer_history[0].entry_type,
        NegotiationHistoryEntryType::Reject
    );

    let row = sqlx::query(
        "SELECT status, final_offer_amount, jsonb_array_length(offer_history) AS history_len
         FROM negotiations
         WHERE negotiation_id = $1",
    )
    .bind(&negotiation_id)
    .fetch_one(&pool)
    .await?;
    let stored_status: String = row.try_get("status")?;
    let stored_final_offer: Option<f64> = row.try_get("final_offer_amount")?;
    let stored_history_len: i32 = row.try_get("history_len")?;
    assert_eq!(stored_status, "cancelled");
    assert_eq!(stored_final_offer, None);
    assert_eq!(stored_history_len, 1);

    Ok(())
}

#[tokio::test]
async fn postgres_negotiation_acceptance_flow_persists_closed_state_and_final_offer(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let negotiation_id = format!("neg_accept_{}", suffix);
    let listing_id = format!("lst_accept_{}", suffix);
    let buyer_id = format!("buyer_accept_{}", suffix);

    // Seed listing (FK dependency for negotiations)
    seed_listing(&pool, &listing_id).await?;

    // Insert initial negotiation
    sqlx::query(
        "INSERT INTO negotiations (negotiation_id, listing_id, buyer_agent_id, status, offer_currency, latest_offer_amount, offer_history, version, open_idempotency_key, created_at, updated_at)
         VALUES ($1, $2, $3, 'open', 'USD', 100.0, '[]'::jsonb, 1, $4, '2026-05-12T00:00:00Z', '2026-05-12T00:00:00Z')"
    )
    .bind(&negotiation_id)
    .bind(&listing_id)
    .bind(&buyer_id)
    .bind(format!("key_accept_{}", suffix))
    .execute(&pool)
    .await?;

    let repo = PostgresNegotiationRepository::new(pool.clone());
    let request = AcceptNegotiationRequest {
        idempotency_key: "accept_key".to_string(),
    };

    // Accept negotiation
    let result = repo
        .accept_negotiation(
            &negotiation_id,
            &request,
            "seller",
            "seller",
            "2026-05-12T00:00:01Z",
        )
        .await?;
    assert_eq!(result.status, NegotiationStatus::Closed);
    assert_eq!(result.final_offer_amount, Some(100.0));
    assert_eq!(result.version, 2);
    assert_eq!(result.offer_history.len(), 1);
    assert_eq!(
        result.offer_history[0].entry_type,
        NegotiationHistoryEntryType::Accept
    );

    // Verify in DB
    let row = sqlx::query("SELECT status, final_offer_amount, jsonb_array_length(offer_history) as history_len FROM negotiations WHERE negotiation_id = $1")
        .bind(&negotiation_id)
        .fetch_one(&pool)
        .await?;
    let stored_status: String = row.try_get("status")?;
    let stored_final_offer: Option<f64> = row.try_get("final_offer_amount")?;
    let stored_history_len: i32 = row.try_get("history_len")?;
    assert_eq!(stored_status, "closed");
    assert_eq!(stored_final_offer, Some(100.0));
    assert_eq!(stored_history_len, 1);

    Ok(())
}

#[tokio::test]
async fn postgres_contact_reveal_request_and_approve_flow(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let reveal_id = format!("rev_req_{}", suffix);
    let negotiation_id = format!("neg_reveal_{}", suffix);
    let listing_id = format!("lst_reveal_{}", suffix);
    let buyer_id = format!("buyer_reveal_{}", suffix);

    // Seed listing and negotiation (FK dependencies for contact_reveals)
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    // Insert reveal request
    sqlx::query(
        "INSERT INTO contact_reveals (reveal_id, negotiation_id, listing_id, buyer_agent_id, reveal_status, revealed_phone_reference, request_idempotency_key, created_at)
         VALUES ($1, $2, $3, $4, 'pending', '+1234567890', $5, '2026-05-12T00:00:00Z')"
    )
    .bind(&reveal_id)
    .bind(&negotiation_id)
    .bind(&listing_id)
    .bind(&buyer_id)
    .bind(format!("req_key_{}", suffix))
    .execute(&pool)
    .await?;

    let repo = PostgresContactRevealRepository::new(pool.clone());

    // Approve reveal
    let result = repo
        .approve_request(&reveal_id, "2026-05-12T00:01:00Z")
        .await?;
    assert_eq!(result.reveal_id, reveal_id);
    assert_eq!(
        result.reveal_status,
        marketplace_api_contract::ContactRevealStatus::Approved
    );

    // Verify in DB
    let row = sqlx::query("SELECT reveal_status FROM contact_reveals WHERE reveal_id = $1")
        .bind(&reveal_id)
        .fetch_one(&pool)
        .await?;
    let status: String = row.try_get("reveal_status")?;
    assert_eq!(status, "approved");

    Ok(())
}

#[tokio::test]
async fn postgres_negotiation_submit_offer_invalid_negotiation_returns_error(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let repo = PostgresNegotiationRepository::new(pool);
    let request = SubmitOfferRequest {
        offer_amount: 200.0,
        offer_currency: "USD".to_string(),
        idempotency_key: "invalid_key".to_string(),
    };

    let result = repo
        .submit_offer(
            "nonexistent_neg",
            &request,
            "user",
            "buyer",
            "2026-05-12T00:00:00Z",
        )
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.kind,
        marketplace_server::repositories::RepositoryErrorKind::NotFound
    );

    Ok(())
}

#[tokio::test]
async fn postgres_reservation_lease_creation_and_expiry() -> Result<(), Box<dyn Error + Send + Sync>>
{
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_lease_{}", suffix);
    let negotiation_id = format!("neg_lease_{}", suffix);

    // Seed listing (FK dependency for reservation_leases)
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

    let repo = PostgresReservationLeaseRepository::new(pool.clone());

    // Create lease (Postgres auto-generates lease_id via sequence)
    let result = repo
        .reserve(
            &listing_id,
            &negotiation_id,
            "user",
            "2026-05-12T00:00:00Z",
            3600,
        )
        .await?;
    assert_eq!(result.listing_id, listing_id);
    let actual_lease_id = result.lease_id.clone();

    // Check lease exists
    let lease: Option<marketplace_server::models::db::ReservationLeaseRow> =
        repo.get_active_by_listing(&listing_id).await?;
    assert!(lease.is_some());

    // Release lease
    repo.release(&actual_lease_id, "2026-05-12T01:00:00Z")
        .await?;

    // Check lease gone
    let lease_after: Option<marketplace_server::models::db::ReservationLeaseRow> =
        repo.get_active_by_listing(&listing_id).await?;
    assert!(lease_after.is_none());

    Ok(())
}

#[tokio::test]
async fn postgres_auth_flow_create_listing_with_valid_seller_role(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();

    let owner = format!("seller_user_{}", suffix);

    // Seed seller account
    sqlx::query(
        "INSERT INTO seller_accounts (seller_account_id, owner_id, trust_level, status, listings_created, quota_override, created_at, updated_at)
         VALUES ($1, $2, 'trusted', 'active', 0, 100, '2026-05-12T00:00:00Z', '2026-05-12T00:00:00Z')"
    )
    .bind(format!("seller_{}", suffix))
    .bind(&owner)
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::PostgresAuditEventRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::PostgresOutboxEventRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    let claims = marketplace_auth_core::Claims {
        sub: owner.clone(),
        roles: vec![marketplace_auth_core::Role::SellerListingWriter],
        scopes: vec![marketplace_auth_core::Scope::ListingCreate],
        seller_account_id: Some(format!("seller_{}", suffix)),
        buyer_agent_id: None,
        hardware_id: None,
        exp: Some(1715475600),
    };

    let request = marketplace_api_contract::CreateListingRequest {
        idempotency_key: format!("create_auth_{}", suffix),
        listing: marketplace_api_contract::ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: format!("seller_{}", suffix),
            listing_type: marketplace_api_contract::ListingType::Product,
            category: Some(marketplace_api_contract::Category::Laptop),
            title: "Auth Test Laptop".to_string(),
            condition: Some(marketplace_api_contract::Condition::New),
            price: marketplace_api_contract::Price {
                currency: "USD".to_string(),
                amount: 1000.0,
            },
            location: marketplace_api_contract::ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "New York".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec!["https://example.com/laptop.jpg".to_string()],
            description: "Test listing for auth".to_string(),
            attributes: None,
            sku: None,
            quantity: None,
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
    };

    let result = app
        .create_listing(
            &claims,
            &request,
            &format!("fp_{}", suffix),
            "2026-05-12T00:00:00Z",
        )
        .await;
    assert!(result.is_ok());
    // Note: listing_id generation might differ, so just check it's created

    Ok(())
}

#[tokio::test]
async fn postgres_seller_account_trust_level_update() -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let seller_id = format!("seller_trust_{}", suffix);

    // Insert seller account
    sqlx::query(
        "INSERT INTO seller_accounts (seller_account_id, owner_id, trust_level, status, listings_created, quota_override, created_at, updated_at)
         VALUES ($1, $2, 'verified', 'active', 0, 100, '2026-05-12T00:00:00Z', '2026-05-12T00:00:00Z')"
    )
    .bind(&seller_id)
    .bind(format!("owner_trust_{}", suffix))
    .execute(&pool)
    .await?;

    let repo =
        marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
            pool.clone(),
        );

    // Update trust level
    let updated = repo
        .update_trust_level(&seller_id, "trusted")
        .await
        .unwrap();
    assert!(updated.is_some());
    let account = updated.unwrap();
    assert_eq!(account.trust_level, "trusted");
    assert_eq!(account.seller_account_id, seller_id);

    // Verify in DB
    let row = sqlx::query("SELECT trust_level FROM seller_accounts WHERE seller_account_id = $1")
        .bind(&seller_id)
        .fetch_one(&pool)
        .await?;
    let db_trust: String = row.try_get("trust_level")?;
    assert_eq!(db_trust, "trusted");

    Ok(())
}

#[tokio::test]
async fn postgres_rejects_outsider_reveal_request() -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_outsider_{suffix}");

    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, 'seller-1', '1.0', 'laptop', 'Outsider Test', 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $2, $3, $4, 'active', 1, $5, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&listing_id)
    .bind(serde_json::json!(["https://example.com/item.jpg"]))
    .bind("Outsider test listing")
    .bind(serde_json::json!({"brand": "Test"}))
    .bind(format!("create-outsider-{suffix}"))
    .execute(&pool)
    .await?;

    let negotiation_id = format!("neg_outsider_{suffix}");
    sqlx::query(
        "INSERT INTO negotiations (
            negotiation_id, listing_id, buyer_agent_id, status, offer_currency,
            latest_offer_amount, reservation_lease_id, final_offer_amount, version,
            open_idempotency_key, created_at, updated_at
        ) VALUES ($1, $2, 'buyer-1', 'reserved', 'USD', 499.00, NULL, NULL, 1, $3, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&negotiation_id)
    .bind(&listing_id)
    .bind(format!("open-outsider-{suffix}"))
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    // Outsider buyer tries to request reveal
    let outsider = marketplace_auth_core::Claims {
        sub: "buyer-2".to_string(),
        roles: vec![marketplace_auth_core::Role::BuyerNegotiator],
        scopes: vec![marketplace_auth_core::Scope::NegotiationRevealRequest],
        seller_account_id: None,
        buyer_agent_id: Some("buyer-2".to_string()),
        hardware_id: None,
        exp: Some(1715475600),
    };

    let result = app
        .request_contact_reveal(
            &outsider,
            &negotiation_id,
            &RequestContactRevealRequest {
                idempotency_key: format!("reveal-outsider-{suffix}"),
            },
            &format!("fp-outsider-{suffix}"),
            "2026-05-04T00:00:00Z",
        )
        .await;
    assert!(
        matches!(
            result,
            Err(marketplace_server::http::handlers::HandlerError::Authz(_))
        ),
        "outsider reveal request should be forbidden, got {:?}",
        result
    );

    Ok(())
}

#[tokio::test]
async fn postgres_rejects_wrong_seller_reveal_approval() -> Result<(), Box<dyn Error + Send + Sync>>
{
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_wrongseller_{suffix}");

    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, 'seller-1', '1.0', 'laptop', 'Wrong Seller Test', 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $2, $3, $4, 'active', 1, $5, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&listing_id)
    .bind(serde_json::json!(["https://example.com/item.jpg"]))
    .bind("Wrong seller approval test")
    .bind(serde_json::json!({"brand": "Test"}))
    .bind(format!("create-wrongseller-{suffix}"))
    .execute(&pool)
    .await?;

    let negotiation_id = format!("neg_wrongseller_{suffix}");
    sqlx::query(
        "INSERT INTO negotiations (
            negotiation_id, listing_id, buyer_agent_id, status, offer_currency,
            latest_offer_amount, reservation_lease_id, final_offer_amount, version,
            open_idempotency_key, created_at, updated_at
        ) VALUES ($1, $2, 'buyer-1', 'reserved', 'USD', 499.00, NULL, NULL, 1, $3, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&negotiation_id)
    .bind(&listing_id)
    .bind(format!("open-wrongseller-{suffix}"))
    .execute(&pool)
    .await?;

    let reveal_id = format!("rev_wrongseller_{suffix}");
    sqlx::query(
        "INSERT INTO contact_reveals (
            reveal_id, negotiation_id, listing_id, buyer_agent_id,
            reveal_status, revealed_phone_reference, request_idempotency_key, created_at
        ) VALUES ($1, $2, $3, 'buyer-1', 'pending', '+1234567890', $4, '2026-05-04T00:00:00Z')",
    )
    .bind(&reveal_id)
    .bind(&negotiation_id)
    .bind(&listing_id)
    .bind(format!("req-wrongseller-{suffix}"))
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    // Wrong seller tries to approve
    let wrong_seller = marketplace_auth_core::Claims {
        sub: "seller-2".to_string(),
        roles: vec![marketplace_auth_core::Role::SellerContactRevealApprover],
        scopes: vec![marketplace_auth_core::Scope::RevealApprove],
        seller_account_id: Some("seller-2".to_string()),
        buyer_agent_id: None,
        hardware_id: None,
        exp: Some(1715475600),
    };

    let result = app.approve_contact_reveal(&wrong_seller, &reveal_id).await;
    assert!(
        matches!(
            result,
            Err(marketplace_server::http::handlers::HandlerError::Authz(_))
        ),
        "wrong seller approval should be forbidden, got {:?}",
        result
    );

    Ok(())
}

#[tokio::test]
async fn postgres_rejects_open_negotiation_invalid_amount(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_invamt_{suffix}");

    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, 'seller-1', '1.0', 'laptop', 'Invalid Amount Test', 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $2, $3, $4, 'active', 1, $5, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&listing_id)
    .bind(serde_json::json!(["https://example.com/item.jpg"]))
    .bind("Invalid amount test listing")
    .bind(serde_json::json!({"brand": "Test"}))
    .bind(format!("create-invamt-{suffix}"))
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    let buyer = marketplace_auth_core::Claims {
        sub: "buyer-1".to_string(),
        roles: vec![marketplace_auth_core::Role::BuyerNegotiator],
        scopes: vec![marketplace_auth_core::Scope::NegotiationCreate],
        seller_account_id: None,
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: Some(1715475600),
    };

    // Zero amount
    let result = app
        .open_negotiation(
            &buyer,
            &marketplace_api_contract::OpenNegotiationRequest {
                listing_id: listing_id.clone(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: 0.0,
                idempotency_key: format!("open-zero-{suffix}"),
            },
            &format!("fp-zero-{suffix}"),
            "2026-05-04T00:00:00Z",
        )
        .await;
    assert!(result.is_err(), "zero offer amount should be rejected");

    // Negative amount
    let result = app
        .open_negotiation(
            &buyer,
            &marketplace_api_contract::OpenNegotiationRequest {
                listing_id: listing_id.clone(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: -100.0,
                idempotency_key: format!("open-neg-{suffix}"),
            },
            &format!("fp-neg-{suffix}"),
            "2026-05-04T00:00:00Z",
        )
        .await;
    assert!(result.is_err(), "negative offer amount should be rejected");

    Ok(())
}

#[tokio::test]
async fn postgres_open_negotiation_conflict_compensation(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_conflict_{suffix}");

    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, 'seller-1', '1.0', 'laptop', 'Conflict Test', 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $2, $3, $4, 'active', 1, $5, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&listing_id)
    .bind(serde_json::json!(["https://example.com/item.jpg"]))
    .bind("Conflict compensation test listing")
    .bind(serde_json::json!({"brand": "Test"}))
    .bind(format!("create-conflict-{suffix}"))
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    let seller = marketplace_auth_core::Claims {
        sub: "buyer-1".to_string(),
        roles: vec![marketplace_auth_core::Role::BuyerNegotiator],
        scopes: vec![marketplace_auth_core::Scope::NegotiationCreate],
        seller_account_id: None,
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: Some(1715475600),
    };

    // First open-negotiation succeeds (creates reservation)
    let result = app
        .open_negotiation(
            &seller,
            &marketplace_api_contract::OpenNegotiationRequest {
                listing_id: listing_id.clone(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: 400.0,
                idempotency_key: format!("open-first-{suffix}"),
            },
            &format!("fp-first-{suffix}"),
            "2026-05-04T00:00:00Z",
        )
        .await;
    assert!(
        result.is_ok(),
        "first open-negotiation should succeed, got {:?}",
        result
    );

    // Second open-negotiation fails with Conflict (listing already reserved)
    let result = app
        .open_negotiation(
            &seller,
            &marketplace_api_contract::OpenNegotiationRequest {
                listing_id: listing_id.clone(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: 450.0,
                idempotency_key: format!("open-second-{suffix}"),
            },
            &format!("fp-second-{suffix}"),
            "2026-05-04T00:01:00Z",
        )
        .await;
    assert!(
        matches!(result, Err(marketplace_server::http::handlers::HandlerError::Repository(ref e)) if e.kind == marketplace_server::repositories::RepositoryErrorKind::Conflict),
        "second open-negotiation should conflict, got {:?}",
        result
    );

    Ok(())
}

#[tokio::test]
async fn postgres_open_negotiation_inactive_listing_commits_idempotency_failure(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(pool) = live_bootstrapped_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let listing_id = format!("lst_inactive_{suffix}");

    sqlx::query(
        "INSERT INTO listings (
            listing_id, owner_id, schema_version, category, product_name, \"condition\",
            price_currency, price_amount, country_code, country_name, city,
            picture_urls, description, attributes, status, version, create_idempotency_key,
            created_at, updated_at
        ) VALUES ($1, 'seller-1', '1.0', 'laptop', 'Inactive Test', 'used', 'USD', 499.00, 'JP', 'Japan', 'Osaka', $2, $3, $4, 'sold', 1, $5, '2026-05-04T00:00:00Z', '2026-05-04T00:00:00Z')",
    )
    .bind(&listing_id)
    .bind(serde_json::json!(["https://example.com/item.jpg"]))
    .bind("Inactive listing test")
    .bind(serde_json::json!({"brand": "Test"}))
    .bind(format!("create-inactive-{suffix}"))
    .execute(&pool)
    .await?;

    let app = marketplace_server::app::MarketplaceApp::new(
        marketplace_server::repositories::listings::PostgresListingRepository::new(pool.clone()),
        marketplace_server::services::idempotency::InMemoryIdempotencyRepository::new(),
        marketplace_server::repositories::reservations::PostgresReservationLeaseRepository::new(
            pool.clone(),
        ),
        marketplace_server::repositories::contact_reveals::PostgresContactRevealRepository::new(
            pool.clone(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::negotiations::PostgresNegotiationRepository::new(
                pool.clone(),
            ),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::audit_events::InMemoryAuditEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository::new(),
        ),
        std::sync::Arc::new(
            marketplace_server::repositories::seller_accounts::PostgresSellerAccountRepository::new(
                pool,
            ),
        ),
    );

    let buyer = marketplace_auth_core::Claims {
        sub: "buyer-1".to_string(),
        roles: vec![marketplace_auth_core::Role::BuyerNegotiator],
        scopes: vec![marketplace_auth_core::Scope::NegotiationCreate],
        seller_account_id: None,
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: Some(1715475600),
    };

    let result = app
        .open_negotiation(
            &buyer,
            &marketplace_api_contract::OpenNegotiationRequest {
                listing_id: listing_id.clone(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: 400.0,
                idempotency_key: format!("open-inactive-{suffix}"),
            },
            &format!("fp-inactive-{suffix}"),
            "2026-05-04T00:00:00Z",
        )
        .await;
    assert!(
        result.is_err(),
        "open-negotiation on sold listing should fail"
    );

    Ok(())
}
