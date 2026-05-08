pub mod audit_events;
pub mod authz;
pub mod contact_reveals;
pub mod idempotency;
pub mod outbox_events;
pub mod search;
pub mod ai_cache;  // NEW: AI prompt caching (Moka-based)

pub const MODULE_NAME: &str = "services";
