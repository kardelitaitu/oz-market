use marketplace_api_contract::{
    AcceptNegotiationRequest, NegotiationHistoryEntryType, NegotiationStatus,
    RejectNegotiationRequest, RequestContactRevealRequest, SubmitOfferRequest,
};
use marketplace_server::repositories::{
    negotiations::{NegotiationRepository, PostgresNegotiationRepository},
    ContactRevealRepository, PostgresContactRevealRepository, PostgresReservationLeaseRepository,
    ReservationLeaseRepository,
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

async fn ensure_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    for migration in [
        include_str!("../migrations/0001_init.sql"),
        include_str!("../migrations/0013_add_negotiation_offer_history.sql"),
    ] {
        for statement in migration.split(';') {
            let statement = statement.trim();
            if statement.is_empty()
                || statement.eq_ignore_ascii_case("BEGIN")
                || statement.eq_ignore_ascii_case("COMMIT")
            {
                continue;
            }
            sqlx::query(statement).execute(pool).await?;
        }
    }
    Ok(())
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
    ensure_schema(&pool).await?;

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
    ensure_schema(&pool).await?;

    let suffix = unique_suffix();
    let listing_id = format!("lst_{suffix}");
    let negotiation_id = format!("neg_{suffix}");
    seed_listing(&pool, &listing_id).await?;
    seed_negotiation(&pool, &negotiation_id, &listing_id).await?;

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
    ensure_schema(&pool).await?;

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
    ensure_schema(&pool).await?;

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
