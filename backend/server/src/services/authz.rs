use crate::auth::{authorize, Action, AuthzError, OwnershipContext};
use marketplace_auth_core::Claims;

pub fn authorize_create_listing(claims: &Claims, owner_id: &str) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: owner_id,
        },
    )
}

pub fn authorize_get_listing(claims: &Claims) -> Result<(), AuthzError> {
    authorize(claims, Action::GetListing, OwnershipContext::None)
}

pub fn authorize_search_listings(claims: &Claims) -> Result<(), AuthzError> {
    authorize(claims, Action::SearchListings, OwnershipContext::None)
}

pub fn authorize_open_negotiation(claims: &Claims, buyer_agent_id: &str) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::OpenNegotiation,
        OwnershipContext::BuyerOwned { buyer_agent_id },
    )
}

pub fn authorize_get_negotiation_status(
    claims: &Claims,
    seller_account_id: &str,
    buyer_agent_id: &str,
) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::GetNegotiationStatus,
        OwnershipContext::NegotiationParticipant {
            seller_account_id,
            buyer_agent_id,
        },
    )
}

pub fn authorize_submit_offer(
    claims: &Claims,
    seller_account_id: &str,
    buyer_agent_id: &str,
) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::SubmitOffer,
        OwnershipContext::NegotiationParticipant {
            seller_account_id,
            buyer_agent_id,
        },
    )
}

pub fn authorize_request_contact_reveal(
    claims: &Claims,
    seller_account_id: &str,
    buyer_agent_id: &str,
) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::RequestContactReveal,
        OwnershipContext::NegotiationParticipant {
            seller_account_id,
            buyer_agent_id,
        },
    )
}

pub fn authorize_approve_contact_reveal(
    claims: &Claims,
    seller_account_id: &str,
) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::ApproveContactReveal,
        OwnershipContext::SellerOwned {
            seller_account_id,
        },
    )
}
