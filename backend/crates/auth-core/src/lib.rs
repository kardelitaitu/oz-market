mod error;
pub use error::AuthError;
pub use error::AuthResult;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SellerListingWriter,
    SellerNegotiator,
    SellerContactRevealApprover,
    BuyerSearcher,
    BuyerNegotiator,
    Admin,
    SupportReviewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    #[serde(rename = "listing:create")]
    ListingCreate,
    #[serde(rename = "listing:read")]
    ListingRead,
    #[serde(rename = "listing:search")]
    ListingSearch,
    #[serde(rename = "negotiation:create")]
    NegotiationCreate,
    #[serde(rename = "negotiation:read")]
    NegotiationRead,
    #[serde(rename = "negotiation:offer:submit")]
    NegotiationOfferSubmit,
    #[serde(rename = "negotiation:reveal:request")]
    NegotiationRevealRequest,
    #[serde(rename = "reveal:approve")]
    RevealApprove,
    // Internal admin scopes
    #[serde(rename = "internal:listing:archive")]
    InternalListingArchive,
    #[serde(rename = "internal:seller:trust-level")]
    InternalSellerTrustLevel,
    #[serde(rename = "internal:seller:quota-override")]
    InternalSellerQuotaOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Claims {
    pub sub: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buyer_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}

impl Claims {
    pub fn has_scope(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.exp {
            let now = chrono::Utc::now().timestamp();
            now >= exp
        } else {
            false
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipContext {
    None,
    SellerOwned {
        seller_account_id: String,
    },
    BuyerOwned {
        buyer_agent_id: String,
    },
    NegotiationParticipant {
        seller_account_id: String,
        buyer_agent_id: String,
    },
}

/// Validate a JWT token and return Claims
pub fn validate_token(token: &str, secret: &[u8]) -> AuthResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // We check manually

    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    let claims = token_data.claims;

    if claims.is_expired() {
        return Err(AuthError::ExpiredToken);
    }

    Ok(claims)
}

/// Extract token from Authorization header
pub fn extract_token(auth_header: &str) -> AuthResult<String> {
    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    match parts.as_slice() {
        ["Bearer", token] => Ok(token.to_string()),
        _ => Err(AuthError::MissingAuthHeader),
    }
}

/// Authorize an action based on claims
pub fn authorize(claims: &Claims, action: Action, ownership: OwnershipContext) -> AuthResult<()> {
    // Check action-specific scopes
    let required_scopes = action_to_scopes(action);
    let claims_scopes: HashSet<_> = claims.scopes.iter().collect();

    let has_scope = required_scopes.iter().any(|s| claims_scopes.contains(s));
    if !has_scope {
        return Err(AuthError::InsufficientScope(
            required_scopes
                .first()
                .copied()
                .unwrap_or(Scope::ListingRead),
        ));
    }

    // Check ownership if applicable
    match ownership {
        OwnershipContext::None => Ok(()),
        OwnershipContext::SellerOwned { seller_account_id } => {
            if let Some(ref sid) = claims.seller_account_id {
                if sid == &seller_account_id {
                    Ok(())
                } else {
                    Err(AuthError::OwnershipFailed)
                }
            } else {
                Err(AuthError::OwnershipFailed)
            }
        }
        OwnershipContext::BuyerOwned { buyer_agent_id } => {
            if let Some(ref bid) = claims.buyer_agent_id {
                if bid == &buyer_agent_id {
                    Ok(())
                } else {
                    Err(AuthError::OwnershipFailed)
                }
            } else {
                Err(AuthError::OwnershipFailed)
            }
        }
        OwnershipContext::NegotiationParticipant {
            seller_account_id,
            buyer_agent_id,
        } => {
            let seller_ok = claims.seller_account_id.as_ref() == Some(&seller_account_id);
            let buyer_ok = claims.buyer_agent_id.as_ref() == Some(&buyer_agent_id);
            if seller_ok || buyer_ok {
                Ok(())
            } else {
                Err(AuthError::OwnershipFailed)
            }
        }
    }
}

pub fn action_to_scopes(action: Action) -> Vec<Scope> {
    match action {
        Action::CreateListing => vec![Scope::ListingCreate],
        Action::GetListing => vec![Scope::ListingRead],
        Action::SearchListings => vec![Scope::ListingSearch],
        Action::OpenNegotiation => vec![Scope::NegotiationCreate],
        Action::GetNegotiationStatus => vec![Scope::NegotiationRead],
        Action::SubmitOffer => vec![Scope::NegotiationOfferSubmit],
        Action::RequestContactReveal => vec![Scope::NegotiationRevealRequest],
        Action::ApproveContactReveal => vec![Scope::RevealApprove],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn claims() -> Claims {
        Claims {
            sub: "agent_123".to_string(),
            roles: vec![Role::SellerListingWriter, Role::BuyerNegotiator],
            scopes: vec![
                Scope::ListingCreate,
                Scope::ListingRead,
                Scope::NegotiationCreate,
            ],
            seller_account_id: Some("seller_123".to_string()),
            buyer_agent_id: Some("buyer_123".to_string()),
            hardware_id: Some("device_123".to_string()),
            exp: Some(1_900_000_000),
        }
    }

    #[test]
    fn extract_token_accepts_bearer_header() {
        let token = extract_token("Bearer abc.def.ghi").unwrap();
        assert_eq!(token, "abc.def.ghi");
    }

    #[test]
    fn extract_token_rejects_malformed_header() {
        assert!(matches!(
            extract_token("Token abc.def.ghi"),
            Err(AuthError::MissingAuthHeader)
        ));
    }

    #[test]
    fn validate_token_accepts_non_expired_claims() {
        let token = encode(
            &Header::default(),
            &Claims {
                exp: Some(1_900_000_000),
                ..claims()
            },
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        let decoded = validate_token(&token, b"secret").unwrap();
        assert_eq!(decoded.sub, "agent_123");
        assert_eq!(decoded.seller_account_id.as_deref(), Some("seller_123"));
    }

    #[test]
    fn validate_token_rejects_expired_claims() {
        let token = encode(
            &Header::default(),
            &Claims {
                exp: Some(1),
                ..claims()
            },
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        assert!(matches!(
            validate_token(&token, b"secret"),
            Err(AuthError::ExpiredToken)
        ));
    }

    #[test]
    fn authorize_requires_scope_for_action() {
        let mut claims = claims();
        claims.scopes.retain(|scope| *scope != Scope::ListingCreate);

        assert!(matches!(
            authorize(
                &claims,
                Action::CreateListing,
                OwnershipContext::SellerOwned {
                    seller_account_id: "seller_123".to_string(),
                }
            ),
            Err(AuthError::InsufficientScope(Scope::ListingCreate))
        ));
    }

    #[test]
    fn authorize_rejects_owner_mismatch_for_seller_owned_action() {
        let claims = claims();

        assert!(matches!(
            authorize(
                &claims,
                Action::CreateListing,
                OwnershipContext::SellerOwned {
                    seller_account_id: "seller_999".to_string(),
                }
            ),
            Err(AuthError::OwnershipFailed)
        ));
    }

    #[test]
    fn authorize_allows_matching_buyer_participant() {
        let claims = claims();

        assert!(authorize(
            &claims,
            Action::OpenNegotiation,
            OwnershipContext::BuyerOwned {
                buyer_agent_id: "buyer_123".to_string(),
            },
        )
        .is_ok());
    }
}
