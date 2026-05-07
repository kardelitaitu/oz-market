pub mod agent_credentials;
pub mod audit_events;
pub mod contact_reveals;
pub mod idempotency_keys;
pub mod listings;
pub mod negotiations;
pub mod outbox_events;
pub mod reservations;
pub mod seller_accounts;

use std::fmt::{Display, Formatter};

pub const MODULE_NAME: &str = "repositories";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryErrorKind {
    Conflict,
    NotFound,
    PermissionDenied,
    Validation,
    Storage,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError {
    pub kind: RepositoryErrorKind,
    pub message: String,
}

impl RepositoryError {
    pub fn new(kind: RepositoryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl Display for RepositoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for RepositoryError {}

pub use agent_credentials::AgentCredentialRepository;
pub use audit_events::AuditEventRepository;
pub use contact_reveals::{
    ContactRevealRepository, InMemoryContactRevealRepository, PostgresContactRevealRepository,
};
pub use idempotency_keys::IdempotencyKeyRepository;
pub use listings::{ListingRepository, InMemoryListingRepository, PostgresListingRepository};
pub use negotiations::{InMemoryNegotiationRepository, NegotiationRepository, PostgresNegotiationRepository};
pub use outbox_events::OutboxEventRepository;
pub use reservations::{
    InMemoryReservationLeaseRepository, PostgresReservationLeaseRepository,
    ReservationLeaseRepository,
};
pub use seller_accounts::{
    InMemorySellerAccountRepository, PostgresSellerAccountRepository, SellerAccountRepository,
};
