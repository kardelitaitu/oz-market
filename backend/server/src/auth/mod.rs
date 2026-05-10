use marketplace_auth_core::{Claims, Role, Scope};

pub use marketplace_auth_core::{Action, OwnershipContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzErrorKind {
    MissingScope,
    MissingRole,
    OwnershipMismatch,
}

impl std::fmt::Display for AuthzErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingScope => write!(f, "MissingScope"),
            Self::MissingRole => write!(f, "MissingRole"),
            Self::OwnershipMismatch => write!(f, "OwnershipMismatch"),
        }
    }
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

impl std::fmt::Display for AuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AuthzError {}

pub fn authorize(
    claims: &Claims,
    action: Action,
    ownership: OwnershipContext,
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
            if actual == Some(seller_account_id.as_str()) || claims.has_role(Role::Admin) {
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
            if actual == Some(buyer_agent_id.as_str()) || claims.has_role(Role::Admin) {
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
            let seller_ok = claims.seller_account_id.as_deref() == Some(seller_account_id.as_str());
            let buyer_ok = claims.buyer_agent_id.as_deref() == Some(buyer_agent_id.as_str());
            if seller_ok || buyer_ok || claims.has_role(Role::Admin) {
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

pub fn authorize_create_listing(claims: &Claims, owner_id: &str) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::CreateListing,
        OwnershipContext::SellerOwned {
            seller_account_id: owner_id.to_string(),
        },
    )
}

pub fn authorize_get_listing(_claims: Option<&Claims>) -> Result<(), AuthzError> {
    // Reading listings is public - no authentication required
    Ok(())
}

pub fn authorize_search_listings(_claims: Option<&Claims>) -> Result<(), AuthzError> {
    // Searching listings is public - no authentication required
    Ok(())
}

pub fn authorize_open_negotiation(claims: &Claims, buyer_agent_id: &str) -> Result<(), AuthzError> {
    authorize(
        claims,
        Action::OpenNegotiation,
        OwnershipContext::BuyerOwned {
            buyer_agent_id: buyer_agent_id.to_string(),
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
            seller_account_id: seller_account_id.to_string(),
            buyer_agent_id: buyer_agent_id.to_string(),
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
            seller_account_id: seller_account_id.to_string(),
        },
    )
}

fn required_scope(action: Action) -> Scope {
    match action {
        Action::CreateListing => Scope::ListingCreate,
        Action::GetListing => Scope::ListingRead,
        Action::SearchListings => Scope::ListingSearch,
        Action::OpenNegotiation => Scope::NegotiationCreate,
        Action::GetNegotiationStatus => Scope::NegotiationRead,
        Action::SubmitOffer => Scope::NegotiationOfferSubmit,
        Action::RequestContactReveal => Scope::NegotiationRevealRequest,
        Action::ApproveContactReveal => Scope::RevealApprove,
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

    fn claims() -> Claims {
        Claims {
            sub: "actor_1".to_string(),
            roles: vec![Role::SellerListingWriter],
            scopes: vec![
                Scope::ListingCreate,
                Scope::ListingRead,
                Scope::NegotiationCreate,
                Scope::NegotiationRead,
                Scope::RevealApprove,
            ],
            seller_account_id: Some("seller_123".to_string()),
            buyer_agent_id: Some("buyer_123".to_string()),
            hardware_id: None,
            exp: None,
        }
    }

    #[test]
    fn authorize_create_listing_allows_matching_seller_writer() {
        assert!(authorize_create_listing(&claims(), "seller_123").is_ok());
    }

    #[test]
    fn authorize_create_listing_rejects_missing_role() {
        let mut claims = claims();
        claims.roles = vec![Role::BuyerSearcher];

        let error = authorize_create_listing(&claims, "seller_123").unwrap_err();
        assert_eq!(error.kind, AuthzErrorKind::MissingRole);
    }

    #[test]
    fn authorize_open_negotiation_rejects_missing_role() {
        let mut claims = claims();
        claims.roles = vec![Role::BuyerSearcher];

        let error = authorize_open_negotiation(&claims, "buyer_123").unwrap_err();
        assert_eq!(error.kind, AuthzErrorKind::MissingRole);
    }

    #[test]
    fn authorize_approve_contact_reveal_allows_admin_on_other_owner() {
        let claims = Claims {
            roles: vec![Role::Admin],
            ..claims()
        };

        assert!(authorize_approve_contact_reveal(&claims, "seller_999").is_ok());
    }
}
