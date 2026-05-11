use marketplace_api_contract::{ListingPayload, ListingType};

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validate a `ListingPayload` against all domain rules derived from the
/// OpenAPI contract. Returns `Ok(())` on success or `Err(Vec<ValidationError>)`
/// collecting every problem found.
pub fn validate_listing_payload(payload: &ListingPayload) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // ── Required fields ──────────────────────────────────────────────
    if payload.owner_id.is_empty() {
        errors.push(ValidationError {
            field: "owner_id".into(),
            message: "owner_id is required and must not be empty".into(),
        });
    }
    if payload.title.is_empty() {
        errors.push(ValidationError {
            field: "title".into(),
            message: "title is required and must not be empty".into(),
        });
    }
    if payload.description.is_empty() {
        errors.push(ValidationError {
            field: "description".into(),
            message: "description is required and must not be empty".into(),
        });
    }

    // ── Field length constraints ─────────────────────────────────────
    if payload.title.len() > 200 {
        errors.push(ValidationError {
            field: "title".into(),
            message: "title must not exceed 200 characters".into(),
        });
    }
    if payload.description.len() > 5000 {
        errors.push(ValidationError {
            field: "description".into(),
            message: "description must not exceed 5000 characters".into(),
        });
    }
    if payload.owner_id.len() > 128 {
        errors.push(ValidationError {
            field: "owner_id".into(),
            message: "owner_id must not exceed 128 characters".into(),
        });
    }

    // ── Price constraints ────────────────────────────────────────────
    if payload.price.amount <= 0.0 {
        errors.push(ValidationError {
            field: "price.amount".into(),
            message: "price.amount must be greater than 0".into(),
        });
    }
    if !payload.price.amount.is_finite() {
        errors.push(ValidationError {
            field: "price.amount".into(),
            message: "price.amount must be a finite number".into(),
        });
    }

    // ── Currency code validation (ISO 4217: 3 uppercase ASCII letters) ─
    let cur = &payload.price.currency;
    if cur.len() != 3 || !cur.chars().all(|c| c.is_ascii_uppercase()) {
        errors.push(ValidationError {
            field: "price.currency".into(),
            message: "currency must be a 3-letter ISO 4217 code (e.g. USD)".into(),
        });
    }

    // ── Picture URLs ─────────────────────────────────────────────────
    if payload.picture_urls.len() > 10 {
        errors.push(ValidationError {
            field: "picture_urls".into(),
            message: "picture_urls must not contain more than 10 items".into(),
        });
    }
    for (i, url) in payload.picture_urls.iter().enumerate() {
        if !is_valid_http_url(url) {
            errors.push(ValidationError {
                field: format!("picture_urls[{i}]"),
                message: "picture URL must be a valid http or https URL".into(),
            });
        }
    }

    // ── Listing-type-specific validation ─────────────────────────────
    match payload.listing_type {
        ListingType::Product => {
            if payload.category.is_none() {
                errors.push(ValidationError {
                    field: "category".into(),
                    message: "category is required for product listings".into(),
                });
            }
            if payload.condition.is_none() {
                errors.push(ValidationError {
                    field: "condition".into(),
                    message: "condition is required for product listings".into(),
                });
            }
        }
        ListingType::Service => {
            if payload.service_type.is_none() {
                errors.push(ValidationError {
                    field: "service_type".into(),
                    message: "service_type is required for service listings".into(),
                });
            }
        }
        ListingType::Property => {
            if payload.property_transaction_type.is_none() {
                errors.push(ValidationError {
                    field: "property_transaction_type".into(),
                    message: "property_transaction_type is required for property listings".into(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn is_valid_http_url(s: &str) -> bool {
    let lowered = s.to_lowercase();
    (lowered.starts_with("http://") || lowered.starts_with("https://"))
        && s.len()
            > lowered
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .len()
                + 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_api_contract::{
        Category, Condition, ListingLocation, ListingType, Price, PropertyTransactionType,
        ServiceType,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn valid_product() -> ListingPayload {
        ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: "owner-1".to_string(),
            listing_type: ListingType::Product,
            category: Some(Category::Laptop),
            title: "Test Laptop".to_string(),
            condition: Some(Condition::New),
            price: Price {
                currency: "USD".to_string(),
                amount: 999.99,
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "New York".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec![],
            description: "A great laptop".to_string(),
            attributes: None,
            sku: None,
            quantity: Some(1),
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: None,
            hourly_rate: None,
            project_rate: None,
            qualifications: None,
            service_radius_km: None,
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        }
    }

    fn errors_for(payload: &ListingPayload) -> Vec<ValidationError> {
        validate_listing_payload(payload).unwrap_err()
    }

    fn assert_has_error(errors: &[ValidationError], field: &str, msg_substr: &str) {
        let found = errors
            .iter()
            .any(|e| e.field == field && e.message.contains(msg_substr));
        assert!(
            found,
            "Expected error for field '{field}' containing '{msg_substr}'.\nGot: {errors:#?}"
        );
    }

    fn assert_valid(payload: &ListingPayload) {
        assert!(
            validate_listing_payload(payload).is_ok(),
            "Expected payload to be valid, but got errors: {:#?}",
            validate_listing_payload(payload).unwrap_err()
        );
    }

    // -----------------------------------------------------------------------
    // 1.3.1 – Price constraints
    // -----------------------------------------------------------------------

    #[test]
    fn price_zero_is_rejected() {
        let mut p = valid_product();
        p.price.amount = 0.0;
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.amount", "greater than 0");
    }

    #[test]
    fn price_negative_is_rejected() {
        let mut p = valid_product();
        p.price.amount = -10.0;
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.amount", "greater than 0");
    }

    #[test]
    fn price_positive_finite_accepted() {
        let mut p = valid_product();
        p.price.amount = 0.01;
        assert_valid(&p);

        p.price.amount = 1_000_000.0;
        assert_valid(&p);

        p.price.amount = f64::MAX;
        assert_valid(&p);
    }

    #[test]
    fn price_nan_is_rejected() {
        let mut p = valid_product();
        p.price.amount = f64::NAN;
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.amount", "finite");
    }

    #[test]
    fn price_infinite_is_rejected() {
        let mut p = valid_product();
        p.price.amount = f64::INFINITY;
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.amount", "finite");
    }

    // -----------------------------------------------------------------------
    // 1.3.2 – Required field validation
    // -----------------------------------------------------------------------

    #[test]
    fn empty_title_rejected() {
        let mut p = valid_product();
        p.title.clear();
        let errs = errors_for(&p);
        assert_has_error(&errs, "title", "required");
    }

    #[test]
    fn empty_owner_id_rejected() {
        let mut p = valid_product();
        p.owner_id.clear();
        let errs = errors_for(&p);
        assert_has_error(&errs, "owner_id", "required");
    }

    #[test]
    fn empty_description_rejected() {
        let mut p = valid_product();
        p.description.clear();
        let errs = errors_for(&p);
        assert_has_error(&errs, "description", "required");
    }

    #[test]
    fn missing_category_rejected_for_product() {
        let mut p = valid_product();
        p.category = None;
        let errs = errors_for(&p);
        assert_has_error(&errs, "category", "required for product");
    }

    #[test]
    fn missing_condition_rejected_for_product() {
        let mut p = valid_product();
        p.condition = None;
        let errs = errors_for(&p);
        assert_has_error(&errs, "condition", "required for product");
    }

    // -----------------------------------------------------------------------
    // 1.3.3 – Field length constraints
    // -----------------------------------------------------------------------

    #[test]
    fn title_max_length_enforced() {
        let mut p = valid_product();
        p.title = "a".repeat(201);
        let errs = errors_for(&p);
        assert_has_error(&errs, "title", "200");
    }

    #[test]
    fn title_exactly_200_accepted() {
        let mut p = valid_product();
        p.title = "a".repeat(200);
        assert_valid(&p);
    }

    #[test]
    fn description_max_length_enforced() {
        let mut p = valid_product();
        p.description = "a".repeat(5001);
        let errs = errors_for(&p);
        assert_has_error(&errs, "description", "5000");
    }

    #[test]
    fn description_exactly_5000_accepted() {
        let mut p = valid_product();
        p.description = "a".repeat(5000);
        assert_valid(&p);
    }

    #[test]
    fn owner_id_max_length_enforced() {
        let mut p = valid_product();
        p.owner_id = "a".repeat(129);
        let errs = errors_for(&p);
        assert_has_error(&errs, "owner_id", "128");
    }

    #[test]
    fn owner_id_exactly_128_accepted() {
        let mut p = valid_product();
        p.owner_id = "a".repeat(128);
        assert_valid(&p);
    }

    // -----------------------------------------------------------------------
    // 1.3.4 – URL validation for picture_urls
    // -----------------------------------------------------------------------

    #[test]
    fn valid_http_url_accepted() {
        let mut p = valid_product();
        p.picture_urls = vec!["http://example.com/photo.jpg".to_string()];
        assert_valid(&p);
    }

    #[test]
    fn valid_https_url_accepted() {
        let mut p = valid_product();
        p.picture_urls = vec!["https://example.com/photo.jpg".to_string()];
        assert_valid(&p);
    }

    #[test]
    fn invalid_url_rejected() {
        let mut p = valid_product();
        p.picture_urls = vec!["not-a-url".to_string()];
        let errs = errors_for(&p);
        assert_has_error(&errs, "picture_urls[0]", "valid http or https URL");
    }

    #[test]
    fn ftp_url_rejected() {
        let mut p = valid_product();
        p.picture_urls = vec!["ftp://example.com/photo.jpg".to_string()];
        let errs = errors_for(&p);
        assert_has_error(&errs, "picture_urls[0]", "valid http or https URL");
    }

    #[test]
    fn too_many_pictures_rejected() {
        let mut p = valid_product();
        p.picture_urls = (0..11)
            .map(|i| format!("https://example.com/photo{i}.jpg"))
            .collect();
        let errs = errors_for(&p);
        assert_has_error(&errs, "picture_urls", "10");
    }

    #[test]
    fn exactly_10_pictures_accepted() {
        let mut p = valid_product();
        p.picture_urls = (0..10)
            .map(|i| format!("https://example.com/photo{i}.jpg"))
            .collect();
        assert_valid(&p);
    }

    // -----------------------------------------------------------------------
    // 1.3.5 – Currency code validation (ISO 4217)
    // -----------------------------------------------------------------------

    #[test]
    fn valid_currency_accepted() {
        let mut p = valid_product();
        p.price.currency = "USD".to_string();
        assert_valid(&p);
        p.price.currency = "EUR".to_string();
        assert_valid(&p);
        p.price.currency = "JPY".to_string();
        assert_valid(&p);
    }

    #[test]
    fn lowercase_currency_rejected() {
        let mut p = valid_product();
        p.price.currency = "usd".to_string();
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.currency", "ISO 4217");
    }

    #[test]
    fn two_letter_currency_rejected() {
        let mut p = valid_product();
        p.price.currency = "US".to_string();
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.currency", "ISO 4217");
    }

    #[test]
    fn four_letter_currency_rejected() {
        let mut p = valid_product();
        p.price.currency = "USDD".to_string();
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.currency", "ISO 4217");
    }

    #[test]
    fn empty_currency_rejected() {
        let mut p = valid_product();
        p.price.currency.clear();
        let errs = errors_for(&p);
        assert_has_error(&errs, "price.currency", "ISO 4217");
    }

    // -----------------------------------------------------------------------
    // 1.3.6 – Listing-type-specific validation
    // -----------------------------------------------------------------------

    #[test]
    fn service_listing_requires_service_type() {
        let mut p = valid_product();
        p.listing_type = ListingType::Service;
        p.category = None;
        p.condition = None;
        p.service_type = None;
        let errs = errors_for(&p);
        assert_has_error(&errs, "service_type", "required for service");
    }

    #[test]
    fn service_listing_with_service_type_accepted() {
        let mut p = valid_product();
        p.listing_type = ListingType::Service;
        p.category = None;
        p.condition = None;
        p.service_type = Some(ServiceType::Online);
        assert_valid(&p);
    }

    #[test]
    fn property_listing_requires_transaction_type() {
        let mut p = valid_product();
        p.listing_type = ListingType::Property;
        p.category = None;
        p.condition = None;
        p.property_transaction_type = None;
        let errs = errors_for(&p);
        assert_has_error(&errs, "property_transaction_type", "required for property");
    }

    #[test]
    fn property_listing_with_transaction_type_accepted() {
        let mut p = valid_product();
        p.listing_type = ListingType::Property;
        p.category = None;
        p.condition = None;
        p.property_transaction_type = Some(PropertyTransactionType::Sale);
        assert_valid(&p);
    }

    // -----------------------------------------------------------------------
    // 1.3.7 – Multiple errors collected at once
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_validation_errors_collected() {
        let p = ListingPayload {
            schema_version: "1.0".to_string(),
            owner_id: String::new(),
            listing_type: ListingType::Product,
            category: None,
            title: String::new(),
            condition: None,
            price: Price {
                currency: "invalid".to_string(),
                amount: -5.0,
            },
            location: ListingLocation {
                country_code: "US".to_string(),
                country_name: "United States".to_string(),
                city: "New York".to_string(),
                latitude: None,
                longitude: None,
                geolocation_opt_out: None,
            },
            picture_urls: vec![],
            description: String::new(),
            attributes: None,
            sku: None,
            quantity: Some(1),
            shipping_info: None,
            condition_details: None,
            seller_notes: None,
            service_type: None,
            hourly_rate: None,
            project_rate: None,
            qualifications: None,
            service_radius_km: None,
            property_transaction_type: None,
            property_sub_type: None,
            area_sqm: None,
            bedrooms: None,
            bathrooms: None,
            year_built: None,
            lot_size_sqm: None,
            zoning: None,
        };
        let errs = errors_for(&p);
        assert!(!errs.is_empty(), "Should have multiple errors");
        assert_has_error(&errs, "owner_id", "required");
        assert_has_error(&errs, "title", "required");
        assert_has_error(&errs, "description", "required");
        assert_has_error(&errs, "price.amount", "greater than 0");
        assert_has_error(&errs, "price.currency", "ISO 4217");
        assert_has_error(&errs, "category", "required for product");
    }

    // -----------------------------------------------------------------------
    // Valid payloads pass
    // -----------------------------------------------------------------------

    #[test]
    fn valid_product_passes() {
        assert_valid(&valid_product());
    }
}
