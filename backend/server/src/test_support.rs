use marketplace_api_contract::{
    Category, Condition, CurrencyCode, ListingLocation, ListingPayload, ListingType,
    OpenNegotiationRequest, Price, ServiceType,
};
use marketplace_auth_core::{Claims, Role, Scope};

// ---------------------------------------------------------------------------
// Default factory helpers
// ---------------------------------------------------------------------------

/// Create a `ListingPayload` with sensible defaults for a product listing.
pub fn make_listing() -> ListingPayload {
    TestListingBuilder::new().build()
}

/// Create a `Claims` with sensible defaults (seller with full capabilities).
pub fn make_user() -> Claims {
    TestUserBuilder::new().build()
}

// ---------------------------------------------------------------------------
// TestListingBuilder
// ---------------------------------------------------------------------------

pub struct TestListingBuilder {
    payload: ListingPayload,
}

impl TestListingBuilder {
    pub fn new() -> Self {
        Self {
            payload: ListingPayload {
                schema_version: "1.0".to_string(),
                owner_id: "owner-1".to_string(),
                listing_type: ListingType::Product,
                category: Some(Category::Laptop),
                title: "Test Listing".to_string(),
                condition: Some(Condition::New),
                price: Price {
                    currency: "USD".to_string(),
                    amount: 100.0,
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
                description: "A test listing description".to_string(),
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
            },
        }
    }

    pub fn build(self) -> ListingPayload {
        self.payload
    }

    pub fn title(mut self, title: &str) -> Self {
        self.payload.title = title.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.payload.description = desc.to_string();
        self
    }

    pub fn owner_id(mut self, id: &str) -> Self {
        self.payload.owner_id = id.to_string();
        self
    }

    pub fn category(mut self, cat: Category) -> Self {
        self.payload.category = Some(cat);
        self
    }

    pub fn condition(mut self, cond: Condition) -> Self {
        self.payload.condition = Some(cond);
        self
    }

    pub fn price(mut self, amount: f64, currency: impl Into<CurrencyCode>) -> Self {
        self.payload.price = Price {
            currency: currency.into(),
            amount,
        };
        self
    }

    pub fn listing_type(mut self, lt: ListingType) -> Self {
        self.payload.listing_type = lt;
        self
    }

    pub fn picture_urls(mut self, urls: Vec<String>) -> Self {
        self.payload.picture_urls = urls;
        self
    }

    pub fn quantity(mut self, qty: u32) -> Self {
        self.payload.quantity = Some(qty);
        self
    }

    pub fn with_location(
        mut self,
        country_code: impl Into<CurrencyCode>,
        country_name: &str,
        city: &str,
    ) -> Self {
        self.payload.location = ListingLocation {
            country_code: country_code.into(),
            country_name: country_name.to_string(),
            city: city.to_string(),
            latitude: None,
            longitude: None,
            geolocation_opt_out: None,
        };
        self
    }

    pub fn with_coordinates(mut self, lat: f64, lng: f64) -> Self {
        self.payload.location.latitude = Some(lat);
        self.payload.location.longitude = Some(lng);
        self
    }

    pub fn geolocation_opt_out(mut self, opt_out: bool) -> Self {
        self.payload.location.geolocation_opt_out = Some(opt_out);
        self
    }

    /// Switch to a Service-type listing with sensible defaults.
    pub fn as_service(mut self) -> Self {
        self.payload.listing_type = ListingType::Service;
        self.payload.service_type = Some(ServiceType::Online);
        self.payload.hourly_rate = Some(50.0);
        self.payload.category = None;
        self.payload.condition = None;
        self
    }

    /// Switch to a Property-type listing with sensible defaults.
    pub fn as_property(mut self) -> Self {
        self.payload.listing_type = ListingType::Property;
        self.payload.category = None;
        self.payload.condition = None;
        self
    }

    /// Attach shipping info.
    pub fn with_shipping(mut self, cost: Option<(f64, impl Into<CurrencyCode>)>) -> Self {
        self.payload.shipping_info = Some(marketplace_api_contract::ShippingInfo {
            local_pickup: true,
            shipping_available: true,
            shipping_cost: cost.map(|(amt, cur)| Price {
                currency: cur.into(),
                amount: amt,
            }),
            shipping_regions: Some(vec!["US".to_string()]),
        });
        self
    }

    pub fn attributes(mut self, attrs: serde_json::Value) -> Self {
        self.payload.attributes = Some(attrs);
        self
    }
}

impl Default for TestListingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TestUserBuilder
// ---------------------------------------------------------------------------

pub struct TestUserBuilder {
    claims: Claims,
}

impl TestUserBuilder {
    pub fn new() -> Self {
        Self {
            claims: Claims {
                sub: "test-user".to_string(),
                roles: vec![
                    Role::SellerListingWriter,
                    Role::BuyerNegotiator,
                    Role::SellerContactRevealApprover,
                ],
                scopes: vec![
                    Scope::ListingCreate,
                    Scope::ListingRead,
                    Scope::ListingSearch,
                    Scope::NegotiationCreate,
                    Scope::NegotiationRead,
                    Scope::NegotiationOfferSubmit,
                    Scope::NegotiationRevealRequest,
                    Scope::RevealApprove,
                ],
                seller_account_id: Some("seller-1".to_string()),
                buyer_agent_id: Some("buyer-1".to_string()),
                hardware_id: None,
                exp: None,
            },
        }
    }

    pub fn build(self) -> Claims {
        self.claims
    }

    pub fn subject(mut self, sub: &str) -> Self {
        self.claims.sub = sub.to_string();
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.claims.roles.push(role);
        self
    }

    pub fn roles(mut self, roles: Vec<Role>) -> Self {
        self.claims.roles = roles;
        self
    }

    pub fn scope(mut self, scope: Scope) -> Self {
        self.claims.scopes.push(scope);
        self
    }

    pub fn scopes(mut self, scopes: Vec<Scope>) -> Self {
        self.claims.scopes = scopes;
        self
    }

    pub fn seller_account_id(mut self, id: &str) -> Self {
        self.claims.seller_account_id = Some(id.to_string());
        self
    }

    pub fn buyer_agent_id(mut self, id: &str) -> Self {
        self.claims.buyer_agent_id = Some(id.to_string());
        self
    }

    pub fn no_seller_id(mut self) -> Self {
        self.claims.seller_account_id = None;
        self
    }

    pub fn no_buyer_id(mut self) -> Self {
        self.claims.buyer_agent_id = None;
        self
    }

    pub fn hardware_id(mut self, id: &str) -> Self {
        self.claims.hardware_id = Some(id.to_string());
        self
    }

    /// Set the token to expire at a past timestamp so `is_expired()` returns true.
    pub fn expired(mut self) -> Self {
        self.claims.exp = Some(1);
        self
    }

    /// Override the expiration timestamp.
    pub fn exp(mut self, ts: i64) -> Self {
        self.claims.exp = Some(ts);
        self
    }

    // -- Convenience presets -------------------------------------------------

    /// Convert to a pure buyer context (no seller roles/ids).
    pub fn as_buyer(mut self) -> Self {
        self.claims.seller_account_id = None;
        self.claims.buyer_agent_id = Some("buyer-1".to_string());
        self.claims.roles = vec![Role::BuyerSearcher, Role::BuyerNegotiator];
        self.claims.scopes = vec![
            Scope::ListingSearch,
            Scope::ListingRead,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
        ];
        self
    }

    /// Convert to an admin context.
    pub fn as_admin(mut self) -> Self {
        self.claims.seller_account_id = None;
        self.claims.buyer_agent_id = None;
        self.claims.roles = vec![Role::Admin];
        self.claims.scopes = vec![Scope::ListingRead, Scope::NegotiationRead];
        self
    }

    /// Convert to a support-reviewer context.
    pub fn as_support(mut self) -> Self {
        self.claims.seller_account_id = None;
        self.claims.buyer_agent_id = None;
        self.claims.roles = vec![Role::SupportReviewer];
        self.claims.scopes = vec![Scope::ListingRead, Scope::NegotiationRead];
        self
    }
}

impl Default for TestUserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TestNegotiationBuilder
// ---------------------------------------------------------------------------

pub struct TestNegotiationBuilder {
    request: OpenNegotiationRequest,
}

impl TestNegotiationBuilder {
    pub fn new() -> Self {
        Self {
            request: OpenNegotiationRequest {
                listing_id: "listing-1".to_string(),
                buyer_agent_id: "buyer-1".to_string(),
                offer_currency: "USD".to_string(),
                offer_amount: 50.0,
                idempotency_key: "neg-key-1".to_string(),
            },
        }
    }

    pub fn build(self) -> OpenNegotiationRequest {
        self.request
    }

    pub fn listing_id(mut self, id: &str) -> Self {
        self.request.listing_id = id.to_string();
        self
    }

    pub fn buyer_agent_id(mut self, id: &str) -> Self {
        self.request.buyer_agent_id = id.to_string();
        self
    }

    pub fn offer(mut self, amount: f64, currency: impl Into<CurrencyCode>) -> Self {
        self.request.offer_amount = amount;
        self.request.offer_currency = currency.into();
        self
    }

    pub fn idempotency_key(mut self, key: &str) -> Self {
        self.request.idempotency_key = key.to_string();
        self
    }
}

impl Default for TestNegotiationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn seller_claims() -> Claims {
    Claims {
        sub: "sub-1".to_string(),
        roles: vec![
            Role::SellerListingWriter,
            Role::BuyerNegotiator,
            Role::SellerContactRevealApprover,
        ],
        scopes: vec![
            Scope::ListingCreate,
            Scope::ListingRead,
            Scope::ListingSearch,
            Scope::NegotiationCreate,
            Scope::NegotiationRead,
            Scope::NegotiationRevealRequest,
            Scope::RevealApprove,
        ],
        seller_account_id: Some("seller-1".to_string()),
        buyer_agent_id: Some("buyer-1".to_string()),
        hardware_id: None,
        exp: None,
    }
}

pub fn admin_claims() -> Claims {
    Claims {
        sub: "admin-1".to_string(),
        roles: vec![Role::Admin],
        scopes: vec![Scope::ListingRead, Scope::NegotiationRead],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}

pub fn support_claims() -> Claims {
    Claims {
        sub: "support-1".to_string(),
        roles: vec![Role::SupportReviewer],
        scopes: vec![Scope::ListingRead, Scope::NegotiationRead],
        seller_account_id: None,
        buyer_agent_id: None,
        hardware_id: None,
        exp: None,
    }
}
