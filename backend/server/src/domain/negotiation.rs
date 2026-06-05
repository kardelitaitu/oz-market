use oz_market_api_contract::listing::ListingStatus;
use oz_market_api_contract::negotiation::NegotiationStatus;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum NegotiationError {
    InvalidOffer {
        field: String,
        message: String,
    },
    InvalidTransition {
        from: NegotiationStatus,
        to: NegotiationStatus,
    },
    ListingNotActive,
    ListingSoldOrArchived,
    OfferExpired,
    CounterOfferOutOfBounds {
        min: f64,
        max: f64,
    },
    ConcurrentNegotiationLimit {
        buyer_id: String,
        seller_id: String,
    },
}

impl std::fmt::Display for NegotiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NegotiationError::InvalidOffer { field, message } => {
                write!(f, "Invalid offer {field}: {message}")
            }
            NegotiationError::InvalidTransition { from, to } => {
                write!(f, "Cannot transition negotiation from {from:?} to {to:?}")
            }
            NegotiationError::ListingNotActive => {
                write!(f, "Listing must be Active to start a negotiation")
            }
            NegotiationError::ListingSoldOrArchived => {
                write!(f, "Cannot negotiate on a sold or archived listing")
            }
            NegotiationError::OfferExpired => {
                write!(f, "Offer has expired")
            }
            NegotiationError::CounterOfferOutOfBounds { min, max } => {
                write!(f, "Counter-offer must be between {min} and {max}")
            }
            NegotiationError::ConcurrentNegotiationLimit {
                buyer_id,
                seller_id,
            } => {
                write!(
                    f,
                    "Buyer {buyer_id} already has an active negotiation with seller {seller_id}"
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Offer validation
// ---------------------------------------------------------------------------

/// Validate that an offer price is positive.
pub fn validate_offer_amount(amount: f64) -> Result<(), NegotiationError> {
    if amount <= 0.0 {
        return Err(NegotiationError::InvalidOffer {
            field: "amount".into(),
            message: "Offer amount must be greater than 0".into(),
        });
    }
    if !amount.is_finite() {
        return Err(NegotiationError::InvalidOffer {
            field: "amount".into(),
            message: "Offer amount must be a finite number".into(),
        });
    }
    Ok(())
}

/// Validate an offer expiration timestamp.  `expires_at` is an RFC 3339 string.
/// Past timestamps are rejected, future timestamps are accepted.
pub fn validate_offer_expiration(expires_at: &str) -> Result<(), NegotiationError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(expires_at).map_err(|_| {
        NegotiationError::InvalidOffer {
            field: "expires_at".into(),
            message: "Invalid RFC 3339 timestamp".into(),
        }
    })?;
    let now = chrono::Utc::now();
    if parsed < now {
        return Err(NegotiationError::OfferExpired);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Counter-offer rules
// ---------------------------------------------------------------------------

/// Validate a counter-offer amount: must be between the previous offer
/// and the listing's asking price (inclusive of both bounds).
pub fn validate_counter_offer(
    previous_offer: f64,
    asking_price: f64,
    counter_amount: f64,
) -> Result<(), NegotiationError> {
    let min = previous_offer.min(asking_price);
    let max = previous_offer.max(asking_price);
    if counter_amount < min || counter_amount > max {
        return Err(NegotiationError::CounterOfferOutOfBounds { min, max });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Listing-status gating
// ---------------------------------------------------------------------------

/// Check that a listing is in a negotiable state (must be Active).
pub fn can_open_negotiation(listing_status: ListingStatus) -> Result<(), NegotiationError> {
    match listing_status {
        ListingStatus::Active => Ok(()),
        ListingStatus::Sold | ListingStatus::Archived => {
            Err(NegotiationError::ListingSoldOrArchived)
        }
        _ => Err(NegotiationError::ListingNotActive),
    }
}

// ---------------------------------------------------------------------------
// Negotiation status transitions
// ---------------------------------------------------------------------------

fn valid_negotiation_transitions(status: NegotiationStatus) -> &'static [NegotiationStatus] {
    use NegotiationStatus::*;
    match status {
        Open => &[Countered, NearClose, Closed, Cancelled],
        Countered => &[NearClose, Closed, Cancelled],
        NearClose => &[Reserved, Closed, Cancelled],
        Reserved => &[ContactRequested, Closed, Cancelled],
        ContactRequested => &[ContactRevealed, Closed, Cancelled],
        ContactRevealed => &[Closed],
        Closed | Cancelled => &[],
    }
}

pub fn validate_negotiation_transition(
    from: NegotiationStatus,
    to: NegotiationStatus,
) -> Result<(), NegotiationError> {
    let valid = valid_negotiation_transitions(from);
    if valid.contains(&to) {
        Ok(())
    } else {
        Err(NegotiationError::InvalidTransition { from, to })
    }
}

/// An offer can only be rejected if it is in a rejectable state
/// (Open, Countered, or NearClose — i.e. not yet locked).
pub fn can_reject_offer(status: NegotiationStatus) -> Result<(), NegotiationError> {
    use NegotiationStatus::*;
    match status {
        Open | Countered | NearClose => Ok(()),
        _ => Err(NegotiationError::InvalidTransition {
            from: status,
            to: NegotiationStatus::Closed,
        }),
    }
}

// ---------------------------------------------------------------------------
// Concurrent negotiation limits
// ---------------------------------------------------------------------------

/// Maximum number of concurrent negotiations a single buyer-seller pair may have.
pub const MAX_CONCURRENT_NEGOTIATIONS_PER_PAIR: usize = 1;

/// Check that a buyer-seller pair has not exceeded the concurrent negotiation limit.
pub fn check_concurrent_limit(
    active_count: usize,
    buyer_id: &str,
    seller_id: &str,
) -> Result<(), NegotiationError> {
    if active_count >= MAX_CONCURRENT_NEGOTIATIONS_PER_PAIR {
        Err(NegotiationError::ConcurrentNegotiationLimit {
            buyer_id: buyer_id.to_string(),
            seller_id: seller_id.to_string(),
        })
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Acceptance rules
// ---------------------------------------------------------------------------

/// An offer can be accepted only when the listing is still Active.
pub fn can_accept_offer(listing_status: ListingStatus) -> Result<(), NegotiationError> {
    if listing_status == ListingStatus::Active {
        Ok(())
    } else {
        Err(NegotiationError::ListingNotActive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NegotiationStatus::*;

    // -----------------------------------------------------------------------
    // 1.5.1 – Offer price must be > 0
    // -----------------------------------------------------------------------

    #[test]
    fn offer_amount_zero_rejected() {
        let err = validate_offer_amount(0.0).unwrap_err();
        assert!(matches!(err, NegotiationError::InvalidOffer { field, .. } if field == "amount"));
    }

    #[test]
    fn offer_amount_negative_rejected() {
        assert!(validate_offer_amount(-10.0).is_err());
    }

    #[test]
    fn offer_amount_nan_rejected() {
        assert!(validate_offer_amount(f64::NAN).is_err());
    }

    #[test]
    fn offer_amount_positive_accepted() {
        assert!(validate_offer_amount(0.01).is_ok());
        assert!(validate_offer_amount(100.0).is_ok());
        assert!(validate_offer_amount(1_000_000.0).is_ok());
    }

    // -----------------------------------------------------------------------
    // 1.5.2 – Offer expiration
    // -----------------------------------------------------------------------

    #[test]
    fn past_expiration_rejected() {
        let past = "2020-01-01T00:00:00Z";
        assert!(matches!(
            validate_offer_expiration(past),
            Err(NegotiationError::OfferExpired)
        ));
    }

    #[test]
    fn future_expiration_accepted() {
        let future = "2030-01-01T00:00:00Z";
        assert!(validate_offer_expiration(future).is_ok());
    }

    #[test]
    fn invalid_timestamp_rejected() {
        let bad = "not-a-date";
        assert!(matches!(
            validate_offer_expiration(bad),
            Err(NegotiationError::InvalidOffer { field, .. }) if field == "expires_at"
        ));
    }

    // -----------------------------------------------------------------------
    // 1.5.3 – Counter-offer rules
    // -----------------------------------------------------------------------

    #[test]
    fn counter_between_offer_and_asking_accepted() {
        // seller asking 500, buyer offers 300 → counter must be 300..500
        assert!(validate_counter_offer(300.0, 500.0, 400.0).is_ok());
        assert!(validate_counter_offer(300.0, 500.0, 300.0).is_ok());
        assert!(validate_counter_offer(300.0, 500.0, 500.0).is_ok());
    }

    #[test]
    fn counter_below_previous_offer_rejected() {
        let err = validate_counter_offer(300.0, 500.0, 200.0).unwrap_err();
        assert!(matches!(
            err,
            NegotiationError::CounterOfferOutOfBounds {
                min: 300.0,
                max: 500.0
            }
        ));
    }

    #[test]
    fn counter_above_asking_price_rejected() {
        let err = validate_counter_offer(300.0, 500.0, 600.0).unwrap_err();
        assert!(matches!(
            err,
            NegotiationError::CounterOfferOutOfBounds {
                min: 300.0,
                max: 500.0
            }
        ));
    }

    #[test]
    fn counter_works_when_buyer_offers_above_asking() {
        // buyer offers 600 on a 500 listing → counter must be 500..600
        assert!(validate_counter_offer(600.0, 500.0, 550.0).is_ok());
        assert!(validate_counter_offer(600.0, 500.0, 500.0).is_ok());
        assert!(validate_counter_offer(600.0, 500.0, 600.0).is_ok());
    }

    // -----------------------------------------------------------------------
    // 1.5.4 – Acceptance requires active listing
    // -----------------------------------------------------------------------

    #[test]
    fn accept_on_active_listing_ok() {
        assert!(can_accept_offer(ListingStatus::Active).is_ok());
    }

    #[test]
    fn accept_on_sold_listing_rejected() {
        assert!(can_accept_offer(ListingStatus::Sold).is_err());
    }

    #[test]
    fn accept_on_archived_listing_rejected() {
        assert!(can_accept_offer(ListingStatus::Archived).is_err());
    }

    #[test]
    fn accept_on_draft_listing_rejected() {
        assert!(can_accept_offer(ListingStatus::Draft).is_err());
    }

    // -----------------------------------------------------------------------
    // 1.5.5 – Rejection rules
    // -----------------------------------------------------------------------

    #[test]
    fn open_offer_can_be_rejected() {
        assert!(can_reject_offer(Open).is_ok());
    }

    #[test]
    fn countered_offer_can_be_rejected() {
        assert!(can_reject_offer(Countered).is_ok());
    }

    #[test]
    fn reserved_offer_cannot_be_rejected() {
        assert!(can_reject_offer(Reserved).is_err());
    }

    #[test]
    fn contact_revealed_offer_cannot_be_rejected() {
        assert!(can_reject_offer(ContactRevealed).is_err());
    }

    #[test]
    fn closed_negotiation_cannot_be_rejected() {
        assert!(can_reject_offer(Closed).is_err());
    }

    #[test]
    fn cancelled_negotiation_cannot_be_rejected() {
        assert!(can_reject_offer(Cancelled).is_err());
    }

    // -----------------------------------------------------------------------
    // 1.5.6 – Negotiation cannot be opened on sold/archived listings
    // -----------------------------------------------------------------------

    #[test]
    fn cannot_open_on_sold_listing() {
        assert!(matches!(
            can_open_negotiation(ListingStatus::Sold),
            Err(NegotiationError::ListingSoldOrArchived)
        ));
    }

    #[test]
    fn cannot_open_on_archived_listing() {
        assert!(matches!(
            can_open_negotiation(ListingStatus::Archived),
            Err(NegotiationError::ListingSoldOrArchived)
        ));
    }

    #[test]
    fn cannot_open_on_draft_listing() {
        assert!(matches!(
            can_open_negotiation(ListingStatus::Draft),
            Err(NegotiationError::ListingNotActive)
        ));
    }

    #[test]
    fn can_open_on_active_listing() {
        assert!(can_open_negotiation(ListingStatus::Active).is_ok());
    }

    // -----------------------------------------------------------------------
    // 1.5.7 – Concurrent negotiation limits per buyer-seller pair
    // -----------------------------------------------------------------------

    #[test]
    fn zero_active_negotiations_allows_new() {
        assert!(check_concurrent_limit(0, "buyer-1", "seller-1").is_ok());
    }

    #[test]
    fn at_limit_rejects_new_negotiation() {
        let err = check_concurrent_limit(1, "buyer-1", "seller-1").unwrap_err();
        assert!(matches!(
            err,
            NegotiationError::ConcurrentNegotiationLimit { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // Negotiation status transition validation
    // -----------------------------------------------------------------------

    #[test]
    fn open_to_countered_valid() {
        assert!(validate_negotiation_transition(Open, Countered).is_ok());
    }

    #[test]
    fn open_to_closed_valid() {
        assert!(validate_negotiation_transition(Open, Closed).is_ok());
    }

    #[test]
    fn countered_to_near_close_valid() {
        assert!(validate_negotiation_transition(Countered, NearClose).is_ok());
    }

    #[test]
    fn near_close_to_reserved_valid() {
        assert!(validate_negotiation_transition(NearClose, Reserved).is_ok());
    }

    #[test]
    fn reserved_to_contact_requested_valid() {
        assert!(validate_negotiation_transition(Reserved, ContactRequested).is_ok());
    }

    #[test]
    fn closed_to_open_invalid() {
        assert!(validate_negotiation_transition(Closed, Open).is_err());
    }

    #[test]
    fn cancelled_to_reserved_invalid() {
        assert!(validate_negotiation_transition(Cancelled, Reserved).is_err());
    }

    #[test]
    fn open_to_reserved_invalid() {
        assert!(validate_negotiation_transition(Open, Reserved).is_err());
    }
}
