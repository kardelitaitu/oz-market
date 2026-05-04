use marketplace_auth_core::{Claims, Role, Scope};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CreateListing,
    GetListing,
    SearchListings,
    OpenNegotiation,
    GetNegotiationStatus,
    SubmitOffer,
    RequestContactReveal,
    ApproveContactReveal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipContext<'a> {
    None,
    SellerOwned {
        seller_account_id: &'a str,
    },
    BuyerOwned {
        buyer_agent_id: &'a str,
    },
    NegotiationParticipant {
        seller_account_id: &'a str,
        buyer_agent_id: &'a str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzErrorKind {
    MissingScope,
    MissingRole,
    OwnershipMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthzError {
    pub kind: AuthzErrorKind,
    pub message: String,
}

impl AuthzError {
    pub fn new(kind: AuthzErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl Display for AuthzError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AuthzError {}

const SCOPE_REQUIRED_LISTING_CREATE: Scope = Scope::ListingCreate;
const SCOPE_REQUIRED_LISTING_READ: Scope = Scope::ListingRead;
const SCOPE_REQUIRED_LISTING_SEARCH: Scope = Scope::ListingSearch;
const SCOPE_REQUIRED_NEGOTIATION_CREATE: Scope = Scope::NegotiationCreate;
const SCOPE_REQUIRED_NEGOTIATION_READ: Scope = Scope::NegotiationRead;
const SCOPE_REQUIRED_NEGOTIATION_OFFER_SUBMIT: Scope = Scope::NegotiationOfferSubmit;
const SCOPE_REQUIRED_NEGOTIATION_REVEAL_REQUEST: Scope = Scope::NegotiationRevealRequest;
const SCOPE_REQUIRED_REVEAL_APPROVE: Scope = Scope::RevealApprove;

pub fn authorize(
    claims: &Claims,
    action: Action,
    ownership: OwnershipContext<'_>,
) -> Result<(), AuthzError> {
    let required_scope = required_scope(action);
    if !claims.has_scope(required_scope) {
        return Err(AuthzError::new(
            AuthzErrorKind::MissingScope,
            format!("missing required scope: {:?}", required_scope),
        ));
    }

    if !claims.has_role(Role::Admin) {
        let allowed_roles = allowed_roles(action);
        if !allowed_roles.iter().any(|role| claims.has_role(*role)) {
            return Err(AuthzError::new(
                AuthzErrorKind::MissingRole,
                format!("missing required role for {:?}", action),
            ));
        }
    }

    match ownership {
        OwnershipContext::None => Ok(()),
        OwnershipContext::SellerOwned { seller_account_id } => {
            let actual = claims.seller_account_id.as_deref();
            if actual == Some(seller_account_id) || claims.has_role(Role::Admin) {
                Ok(())
            } else {
                Err(AuthzError::new(
                    AuthzErrorKind::OwnershipMismatch,
                    "caller does not own the seller account",
                ))
            }
        }
        OwnershipContext::BuyerOwned { buyer_agent_id } => {
            let actual = claims.buyer_agent_id.as_deref();
            if actual == Some(buyer_agent_id) || claims.has_role(Role::Admin) {
                Ok(())
            } else {
                Err(AuthzError::new(
                    AuthzErrorKind::OwnershipMismatch,
                    "caller does not own the buyer agent context",
                ))
            }
        }
        OwnershipContext::NegotiationParticipant {
            seller_account_id,
            buyer_agent_id,
        } => {
            let seller_ok = claims.seller_account_id.as_deref() == Some(seller_account_id);
            let buyer_ok = claims.buyer_agent_id.as_deref() == Some(buyer_agent_id);
            if (seller_ok || buyer_ok || claims.has_role(Role::Admin)) && participant_role_ok(claims, action) {
                Ok(())
            } else {
                Err(AuthzError::new(
                    AuthzErrorKind::OwnershipMismatch,
                    "caller is not an authorized negotiation participant",
                ))
            }
        }
    }
}

fn participant_role_ok(claims: &Claims, action: Action) -> bool {
    if claims.has_role(Role::Admin) {
        return true;
    }

    allowed_roles(action)
        .iter()
        .any(|role| claims.has_role(*role))
}

fn required_scope(action: Action) -> Scope {
    match action {
        Action::CreateListing => SCOPE_REQUIRED_LISTING_CREATE,
        Action::GetListing => SCOPE_REQUIRED_LISTING_READ,
        Action::SearchListings => SCOPE_REQUIRED_LISTING_SEARCH,
        Action::OpenNegotiation => SCOPE_REQUIRED_NEGOTIATION_CREATE,
        Action::GetNegotiationStatus => SCOPE_REQUIRED_NEGOTIATION_READ,
        Action::SubmitOffer => SCOPE_REQUIRED_NEGOTIATION_OFFER_SUBMIT,
        Action::RequestContactReveal => SCOPE_REQUIRED_NEGOTIATION_REVEAL_REQUEST,
        Action::ApproveContactReveal => SCOPE_REQUIRED_REVEAL_APPROVE,
    }
}

fn allowed_roles(action: Action) -> &'static [Role] {
    match action {
        Action::CreateListing => &[Role::SellerListingWriter, Role::Admin],
        Action::GetListing => &[
            Role::SellerListingWriter,
            Role::SellerNegotiator,
            Role::SellerContactRevealApprover,
            Role::BuyerSearcher,
            Role::BuyerNegotiator,
            Role::Admin,
            Role::SupportReviewer,
        ],
        Action::SearchListings => &[
            Role::SellerListingWriter,
            Role::SellerNegotiator,
            Role::SellerContactRevealApprover,
            Role::BuyerSearcher,
            Role::BuyerNegotiator,
            Role::Admin,
            Role::SupportReviewer,
        ],
        Action::OpenNegotiation => &[Role::BuyerNegotiator, Role::Admin],
        Action::GetNegotiationStatus => &[
            Role::SellerNegotiator,
            Role::SellerContactRevealApprover,
            Role::BuyerNegotiator,
            Role::Admin,
            Role::SupportReviewer,
        ],
        Action::SubmitOffer => &[Role::SellerNegotiator, Role::BuyerNegotiator, Role::Admin],
        Action::RequestContactReveal => {
            &[Role::SellerNegotiator, Role::BuyerNegotiator, Role::Admin]
        }
        Action::ApproveContactReveal => &[Role::SellerContactRevealApprover, Role::Admin],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(roles: Vec<Role>, scopes: Vec<Scope>, seller: Option<&str>, buyer: Option<&str>) -> Claims {
        Claims {
            subject: "sub-1".to_string(),
            roles,
            scopes,
            seller_account_id: seller.map(ToString::to_string),
            buyer_agent_id: buyer.map(ToString::to_string),
            hardware_id: None,
            expires_at: None,
        }
    }

    #[test]
    fn create_listing_requires_scope_role_and_ownership() {
        let ok = claims(
            vec![Role::SellerListingWriter],
            vec![Scope::ListingCreate],
            Some("seller-1"),
            None,
        );
        assert!(authorize(&ok, Action::CreateListing, OwnershipContext::SellerOwned { seller_account_id: "seller-1" }).is_ok());

        let bad_scope = claims(
            vec![Role::SellerListingWriter],
            vec![],
            Some("seller-1"),
            None,
        );
        assert!(matches!(
            authorize(&bad_scope, Action::CreateListing, OwnershipContext::SellerOwned { seller_account_id: "seller-1" }),
            Err(AuthzError { kind: AuthzErrorKind::MissingScope, .. })
        ));
    }

    #[test]
    fn buyer_open_negotiation_requires_buyer_context() {
        let ok = claims(
            vec![Role::BuyerNegotiator],
            vec![Scope::NegotiationCreate],
            None,
            Some("buyer-7"),
        );
        assert!(authorize(&ok, Action::OpenNegotiation, OwnershipContext::BuyerOwned { buyer_agent_id: "buyer-7" }).is_ok());

        let wrong_buyer = claims(
            vec![Role::BuyerNegotiator],
            vec![Scope::NegotiationCreate],
            None,
            Some("buyer-8"),
        );
        assert!(matches!(
            authorize(&wrong_buyer, Action::OpenNegotiation, OwnershipContext::BuyerOwned { buyer_agent_id: "buyer-7" }),
            Err(AuthzError { kind: AuthzErrorKind::OwnershipMismatch, .. })
        ));
    }

    #[test]
    fn admin_can_bypass_ownership_but_not_scope() {
        let admin = claims(vec![Role::Admin], vec![Scope::RevealApprove], None, None);
        assert!(authorize(&admin, Action::ApproveContactReveal, OwnershipContext::SellerOwned { seller_account_id: "seller-9" }).is_ok());

        let missing_scope = claims(vec![Role::Admin], vec![], None, None);
        assert!(matches!(
            authorize(&missing_scope, Action::ApproveContactReveal, OwnershipContext::SellerOwned { seller_account_id: "seller-9" }),
            Err(AuthzError { kind: AuthzErrorKind::MissingScope, .. })
        ));
    }
}
