# Internal API Spec (v1)

Frozen internal operational surface for admin and support staff.

## Status

- **Base path**: `/internal/v1`
- **Content type**: `application/json`
- **Auth**: Bearer token with privileged internal scopes
- **Implementation**: ✅ Complete in `backend/server/src/http/runtime.rs`

---

## Boundary Rules

1. Internal routes are **separate from public client contract**
2. Internal routes **reuse the same core business logic** as public routes
3. Internal routes require **separate privileged scopes**
4. Every internal write **requires explicit audit metadata** (reason, requested_by)

---

## Authentication & Authorization

### Token Structure

```json
{
  "sub": "admin-1",
  "roles": ["admin"],
  "scopes": ["support:read", "admin:moderate", "admin:override"],
  "seller_account_id": null,
  "buyer_agent_id": null,
  "hardware_id": null,
  "exp": null
}
```

### Internal Scopes

| Intent | Scope Name | Required For |
|--------|------------|---------------|
| Operational read | `support:read` | GET endpoints |
| Trust/quota override | `admin:moderate` | POST trust-level, quota-override |
| State override | `admin:override` | POST archive, release-reservation, revoke |

### Auth Checks (in `runtime.rs`)

- `authorize_internal_read(&claims)` → requires `support:read` OR `admin:moderate` OR `admin:override`
- `authorize_internal_write(&claims)` → requires `admin:moderate` OR `admin:override`

---

## Endpoints

### 1. Read Operations

#### 1.1 Get Internal Listing

**Endpoint**: `GET /internal/v1/listings/{listing_id}`

**Auth**: `support:read` (or higher)

**Response**: Full `ListingSummary` (same as public GET /v1/listings/{listing_id})

**HTTP Handler** (in `runtime.rs:307-318`):
```rust
("GET", path) if path.starts_with("/internal/v1/listings/") => {
    if let Err(response) = authorize_internal_read(&claims) {
        return response;
    }
    let listing_id = path.trim_start_matches("/internal/v1/listings/");
    match app.get_listing(&claims, listing_id).await {
        Ok(Some(listing)) => json_response(200, serde_json::to_value(listing).unwrap()),
        Ok(None) => api_error_response(404, ApiErrorCode::NotFound, "listing not found", None),
        Err(error) => map_handler_error(&error),
    }
}
```

---

#### 1.2 Get Internal Negotiation

**Endpoint**: `GET /internal/v1/negotiations/{negotiation_id}`

**Auth**: `support:read` (or higher)

**Response**: Full `NegotiationResponse`

**HTTP Handler** (in `runtime.rs:320-329`):
```rust
("GET", path) if path.starts_with("/internal/v1/negotiations/") => {
    if let Err(response) = authorize_internal_read(&claims) {
        return response;
    }
    let negotiation_id = path.trim_start_matches("/internal/v1/negotiations/");
    match app.get_negotiation_status(&claims, negotiation_id).await {
        Ok(response) => json_response(200, serde_json::to_value(response).unwrap()),
        Err(error) => map_handler_error(&error),
    }
}
```

---

#### 1.3 Get Internal Contact Reveal

**Endpoint**: `GET /internal/v1/contact-reveals/{reveal_id}`

**Auth**: `support:read` (or higher)

**Response**: Full `ContactRevealResponse`

**HTTP Handler** (in `runtime.rs:330-345`):
```rust
("GET", path) if path.starts_with("/internal/v1/contact-reveals/") => {
    if let Err(response) = authorize_internal_read(&claims) {
        return response;
    }
    let reveal_id = path.trim_start_matches("/internal/v1/contact-reveals/");
    match app.get_contact_reveal(reveal_id).await {
        Ok(Some(response)) => json_response(200, serde_json::to_value(response).unwrap()),
        Ok(None) => api_error_response(404, ApiErrorCode::NotFound, "contact reveal not found", None),
        Err(error) => map_handler_error(&error),
    }
}
```

---

### 2. Override Operations

#### 2.1 Archive Listing (Admin Override)

**Endpoint**: `POST /internal/v1/listings/{listing_id}/archive`

**Auth**: `admin:moderate` or `admin:override`

**Request Body**:
```json
{
  "reason": "manual fraud review outcome",
  "requested_by": "admin_user_123"
}
```

**Response**: Updated `ListingSummary` with `status: "archived"`

**HTTP Handler** (in `app.rs`):
- Validates seller account ownership
- Sets listing status to `Archived`
- Records audit event with `reason` and `requested_by`
- Records outbox event

---

#### 2.2 Release Reservation (Admin Override)

**Endpoint**: `POST /internal/v1/listings/{listing_id}/release-reservation`

**Auth**: `admin:moderate` or `admin:override`

**Request Body**:
```json
{
  "reason": "admin cleanup",
  "requested_by": "admin_user_123"
}
```

**Response**: Updated `ListingSummary` with reservation released

**Behavior**:
- Finds active reservation lease for listing
- Releases the lease
- Sets negotiation status to `Cancelled`
- Records audit event with before/after state

---

#### 2.3 Set Seller Trust Level

**Endpoint**: `POST /internal/v1/sellers/{seller_account_id}/trust-level`

**Auth**: `admin:moderate`

**Request Body**:
```json
{
  "trust_level": "verified",
  "reason": "passed verification checks",
  "requested_by": "admin_user_123"
}
```

**Valid Trust Levels**: `new`, `verified`, `trusted`, `restricted`

**Response**: Updated `SellerAccount` object

**App Method** (in `app.rs:1057-1111`):
```rust
pub async fn set_seller_trust_level(
    &self,
    claims: &Claims,
    seller_account_id: &str,
    trust_level: &str,
    reason: &str,
    now_rfc3339: &str,
) -> Result<Option<SellerAccount>, HandlerError>
```

---

#### 2.4 Set Seller Quota Override

**Endpoint**: `POST /internal/v1/sellers/{seller_account_id}/quota-override`

**Auth**: `admin:moderate`

**Request Body**:
```json
{
  "quota_override": 100,
  "reason": "temporary increase for promotion",
  "requested_by": "admin_user_123"
}
```

**Behavior**:
- `quota_override: null` → clears override (use default quota)
- `quota_override: <number>` → sets custom quota limit

**Response**: Updated `SellerAccount` object

---

#### 2.5 Revoke Agent Credential (Planned)

**Endpoint**: `POST /internal/v1/agent-credentials/{agent_credential_id}/revoke`

**Auth**: `admin:override`

**Status**: 🔜 Planned, not yet implemented

**Request Body**:
```json
{
  "reason": "compromised credential",
  "requested_by": "admin_user_123"
}
```

---

## Request/Response Shapes

### Override Request Shape (All POST Overrides)

```json
{
  "reason": "string (required)",
  "requested_by": "string (optional, extracted from claims.sub if omitted)"
}
```

### Override Response Shape

All override endpoints return the updated resource:
- Archive → `ListingSummary`
- Release Reservation → `ListingSummary`
- Trust Level → `SellerAccount`
- Quota Override → `SellerAccount`

---

## Audit Requirements

### Mandatory Audit Metadata

Every internal write **must** include:
1. `reason` (string): Why the override was performed
2. `requested_by` (string): Who requested it (usually `claims.sub`)
3. Before/after state (for state changes)

### Audit Event Structure

```json
{
  "event_type": "listing.archived",
  "entity_type": "listing",
  "entity_id": "lst_123",
  "payload": {
    "listing_id": "lst_123",
    "old_status": "active",
    "new_status": "archived",
    "reason": "manual fraud review outcome",
    "requested_by": "admin-1"
  },
  "timestamp": "2026-05-06T05:41:00Z"
}
```

### Audit Repository (in `app.rs`)

All internal writes call:
```rust
self.record_audit_event(
    "listing.archived",
    "listing",
    &listing_id,
    json!({ ... }),
    now_rfc3339,
);
```

---

## Guardrails

### Support Reviewers (Read-Only by Default)

- Have `support:read` scope
- **Can**: Access all GET /internal/v1 endpoints
- **Cannot**: Perform any POST override operations

### Admin Moderate (Trust/Quota)

- Have `admin:moderate` scope
- **Can**: Read internal endpoints + trust-level + quota-override
- **Cannot**: Archive listings or release reservations (requires `admin:override`)

### Admin Override (Full Access)

- Have `admin:override` scope
- **Can**: All internal read + all override operations

---

## Observability

### Internal Route Tracking (in `observability.rs`)

```rust
if path.starts_with("/internal/v1/") {
    // Internal route hit
    self.internal_requests += 1;
}
```

### Metrics Captured
- `internal_requests`: Count of all internal API hits
- `internal_writes`: Count of internal write operations (archive, release, trust-level, quota)
- `conflict_responses`: Count of 409 Conflict responses on internal routes

---

## Error Responses

All internal endpoints return the same error format as public API:

```json
{
  "error": {
    "code": "forbidden",
    "message": "missing required scope for InternalListingArchive",
    "field": null
  }
}
```

### Common Internal Errors

| Code | Message | HTTP Status |
|------|---------|-------------|
| `forbidden` | missing required scope for... | 403 |
| `not_found` | listing/seller/negotiation not found | 404 |
| `conflict` | reservation required before contact reveal | 409 |
| `invalid_field` | reason is required | 400 |

---

## Differences from Public API

| Aspect | Public API | Internal API |
|--------|------------|--------------|
| Base path | `/v1` | `/internal/v1` |
| Auth scopes | `listing:read`, `negotiation:create`, etc. | `support:read`, `admin:moderate`, `admin:override` |
| State transitions | Normal (respects state machine) | Override (can force archive, release) |
| Audit requirements | Optional metadata | **Mandatory** `reason` + `requested_by` |
| Response detail | Standard response | Full internal state (including fields hidden from public) |

---

## Implementation Status

### ✅ Implemented (Tested & Working)

1. ✅ `GET /internal/v1/listings/{listing_id}` (runtime.rs:307)
2. ✅ `GET /internal/v1/negotiations/{negotiation_id}` (runtime.rs:320)
3. ✅ `GET /internal/v1/contact-reveals/{reveal_id}` (runtime.rs:330)
4. ✅ `POST /internal/v1/listings/{listing_id}/archive` (app.rs)
5. ✅ `POST /internal/v1/listings/{listing_id}/release-reservation` (app.rs)
6. ✅ `POST /internal/v1/sellers/{seller_account_id}/trust-level` (app.rs:1057)
7. ✅ `POST /internal/v1/sellers/{seller_account_id}/quota-override` (app.rs:1113)

### 🔜 Planned (Not Yet Implemented)

1. 🔜 `GET /internal/v1/sellers/{seller_account_id}` (outline only)
2. 🔜 `GET /internal/v1/audit-events/{entity_type}/{entity_id}` (outline only)
3. 🔜 `POST /internal/v1/agent-credentials/{agent_credential_id}/revoke` (outline only)

---

## Cross-Reference

- **Public API Spec**: `../openapi.yaml` (frozen)
- **Internal Outline**: `./internal-api-outline.md` (planning doc)
- **Server Implementation**: `backend/server/src/http/runtime.rs` (HTTP layer)
- **Business Logic**: `backend/server/src/app.rs` (shared app layer)
- **Auth Scopes**: `backend/crates/auth-core/src/lib.rs` (scope definitions)
- **Audit Events**: `backend/server/src/repositories/audit_events.rs`
- **Observability**: `backend/server/src/observability/mod.rs`

---

## Maintenance

When adding new internal endpoints:

1. Add route handler in `runtime.rs` with `authorize_internal_read/write` check
2. Implement business logic in `app.rs` with audit/outbox events
3. Update this spec document
4. Add integration test in `runtime.rs` `#[cfg(test)]` module
5. Ensure `observability.rs` tracks the new internal route

---

**Last Updated**: 2026-05-06 (session: Option B - Internal /internal/v1 spec)
**Status**: ✅ Complete for implemented endpoints, 🔜 Outline only for planned endpoints
