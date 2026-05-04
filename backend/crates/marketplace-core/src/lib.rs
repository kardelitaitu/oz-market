#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStage {
    Listing,
    Negotiation,
    Reservation,
    Reveal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    Review,
}

pub const CRATE_NAME: &str = "marketplace-core";
