# Implementation Notes - Cache Invalidation and Admin Interventions

## Actix Web Request Payload Structs

```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AdjustCreditsRequest {
    pub adjustment: String, // "deposit", "spend", "refund", "adjustment"
    pub amount: Decimal,
    pub idempotency_key: String,
}
```

## Actix Web Route Configuration

```rust
use actix_web::{web, HttpResponse, Responder};
use uuid::Uuid;

pub fn configure_admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/admin")
            // Ensure authorization middleware is hooked here
            .route("/sellers/{id}/credits", web::post().to(admin_adjust_credits)),
    );
}
```

## Precise Controller Handler Implementation

```rust
use actix_web::web::{Data, Json, Path};

pub async fn admin_adjust_credits(
    state: Data<AppState>,
    path: Path<Uuid>,
    body: Json<AdjustCreditsRequest>,
) -> impl Responder {
    let agent_id = path.into_inner();
    
    // Parse transaction details
    let tx_type = match body.adjustment.as_str() {
        "deposit" => TransactionType::Deposit,
        "spend" => TransactionType::Spend,
        "refund" => TransactionType::Refund,
        "adjustment" => TransactionType::Adjustment,
        _ => return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "invalid_transaction_type",
            "message": "Transaction type must be deposit, spend, refund, or adjustment."
        })),
    };

    let new_tx = NewTransaction {
        id: Uuid::new_v4(),
        agent_id,
        amount: body.amount,
        tx_type,
        idempotency_key: body.idempotency_key.clone(),
    };

    // Apply database updates and invalidate cache in write-through manner
    match state.ledger_cache.apply_transaction(&new_tx).await {
        Ok(new_balance) => {
            HttpResponse::Ok().json(serde_json::json!({
                "agent_id": agent_id,
                "balance_credits": new_balance,
                "idempotency_key": body.idempotency_key,
                "updated_at": chrono::Utc::now().to_rfc3339()
            }))
        }
        Err(err) => match err {
            CreditLedgerError::InsufficientCredits { requested, available } => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "insufficient_credits",
                    "message": format!("Insufficient credits: requested {requested}, available {available}")
                }))
            }
            CreditLedgerError::DuplicateIdempotencyKey(key) => {
                HttpResponse::Conflict().json(serde_json::json!({
                    "error": "duplicate_idempotency_key",
                    "message": format!("Transaction with idempotency key '{key}' already exists")
                }))
            }
            CreditLedgerError::AgentNotFound(id) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "agent_not_found",
                    "message": format!("Agent {id} not found")
                }))
            }
            CreditLedgerError::DatabaseError(msg) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "internal_error",
                    "message": msg
                }))
            }
        }
    }
}
```
