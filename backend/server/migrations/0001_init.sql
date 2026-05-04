BEGIN;

CREATE TABLE IF NOT EXISTS seller_accounts (
    seller_account_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL UNIQUE,
    trust_level TEXT NOT NULL CHECK (trust_level IN ('new', 'verified', 'trusted', 'restricted')),
    status TEXT NOT NULL CHECK (status IN ('active', 'review', 'suspended')),
    hardware_fingerprint TEXT,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_credentials (
    credential_id TEXT PRIMARY KEY,
    seller_account_id TEXT NOT NULL REFERENCES seller_accounts (seller_account_id) ON DELETE CASCADE,
    subject TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL,
    scopes JSONB NOT NULL DEFAULT '[]'::jsonb,
    revoked_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS listings (
    listing_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    schema_version TEXT NOT NULL DEFAULT '1.0',
    category TEXT NOT NULL CHECK (category IN ('laptop', 'phone', 'tablet', 'desktop', 'monitor', 'accessory', 'camera', 'audio', 'gaming', 'appliance', 'furniture', 'vehicle_part', 'other')),
    product_name TEXT NOT NULL,
    "condition" TEXT NOT NULL CHECK ("condition" IN ('new', 'used', 'refurbished')),
    price_currency CHAR(3) NOT NULL,
    price_amount NUMERIC(20, 2) NOT NULL CHECK (price_amount > 0),
    country_code CHAR(2) NOT NULL,
    country_name TEXT NOT NULL,
    city TEXT NOT NULL,
    picture_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
    description TEXT NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'reserved', 'sold', 'archived')),
    version BIGINT NOT NULL DEFAULT 1,
    create_idempotency_key TEXT NOT NULL UNIQUE,
    search_text TSVECTOR GENERATED ALWAYS AS (
        to_tsvector(
            'simple',
            concat_ws(' ', product_name, description, category, country_name, city, "condition")
        )
    ) STORED,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_listings_owner_id ON listings (owner_id);
CREATE INDEX IF NOT EXISTS idx_listings_status ON listings (status);
CREATE INDEX IF NOT EXISTS idx_listings_category_status ON listings (category, status);
CREATE INDEX IF NOT EXISTS idx_listings_location ON listings (country_code, city);
CREATE INDEX IF NOT EXISTS idx_listings_price ON listings (price_currency, price_amount);
CREATE INDEX IF NOT EXISTS idx_listings_search_text ON listings USING GIN (search_text);

CREATE TABLE IF NOT EXISTS negotiations (
    negotiation_id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL REFERENCES listings (listing_id) ON DELETE CASCADE,
    buyer_agent_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'countered', 'near_close', 'reserved', 'contact_requested', 'contact_revealed', 'closed', 'cancelled')),
    offer_currency CHAR(3) NOT NULL,
    latest_offer_amount NUMERIC(20, 2) NOT NULL CHECK (latest_offer_amount > 0),
    reservation_lease_id TEXT UNIQUE,
    final_offer_amount NUMERIC(20, 2) CHECK (final_offer_amount > 0),
    version BIGINT NOT NULL DEFAULT 1,
    open_idempotency_key TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_negotiations_listing_id ON negotiations (listing_id);
CREATE INDEX IF NOT EXISTS idx_negotiations_buyer_agent_id ON negotiations (buyer_agent_id);
CREATE INDEX IF NOT EXISTS idx_negotiations_status ON negotiations (status);

CREATE TABLE IF NOT EXISTS reservation_leases (
    lease_id TEXT PRIMARY KEY,
    negotiation_id TEXT NOT NULL UNIQUE REFERENCES negotiations (negotiation_id) ON DELETE CASCADE,
    listing_id TEXT NOT NULL REFERENCES listings (listing_id) ON DELETE CASCADE,
    reserved_by TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'expired', 'cancelled')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_reservation_leases_listing_status ON reservation_leases (listing_id, status);
CREATE INDEX IF NOT EXISTS idx_reservation_leases_expires_at ON reservation_leases (expires_at);

CREATE TABLE IF NOT EXISTS contact_reveals (
    reveal_id TEXT PRIMARY KEY,
    negotiation_id TEXT NOT NULL UNIQUE REFERENCES negotiations (negotiation_id) ON DELETE CASCADE,
    listing_id TEXT NOT NULL REFERENCES listings (listing_id) ON DELETE CASCADE,
    buyer_agent_id TEXT NOT NULL,
    request_idempotency_key TEXT NOT NULL UNIQUE,
    reveal_status TEXT NOT NULL CHECK (reveal_status IN ('pending', 'approved', 'rejected', 'expired')),
    revealed_phone_reference TEXT,
    expires_at TIMESTAMPTZ,
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_contact_reveals_listing_id ON contact_reveals (listing_id);
CREATE INDEX IF NOT EXISTS idx_contact_reveals_buyer_agent_id ON contact_reveals (buyer_agent_id);
CREATE INDEX IF NOT EXISTS idx_contact_reveals_status ON contact_reveals (reveal_status);

CREATE TABLE IF NOT EXISTS audit_events (
    event_id BIGSERIAL PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_subject TEXT NOT NULL,
    actor_role TEXT NOT NULL,
    scopes JSONB NOT NULL DEFAULT '[]'::jsonb,
    request_id TEXT,
    idempotency_key TEXT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_events_entity ON audit_events (entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_actor_subject ON audit_events (actor_subject);
CREATE INDEX IF NOT EXISTS idx_audit_events_created_at ON audit_events (created_at DESC);

CREATE TABLE IF NOT EXISTS outbox_events (
    event_id BIGSERIAL PRIMARY KEY,
    topic TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    payload JSONB NOT NULL,
    available_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_outbox_events_pending ON outbox_events (published_at, available_at);
CREATE INDEX IF NOT EXISTS idx_outbox_events_aggregate ON outbox_events (aggregate_type, aggregate_id);

CREATE TABLE IF NOT EXISTS idempotency_keys (
    idempotency_key TEXT NOT NULL,
    actor_subject TEXT NOT NULL,
    operation TEXT NOT NULL,
    request_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'succeeded', 'failed')),
    response_payload JSONB,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (idempotency_key, actor_subject, operation)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_actor_operation ON idempotency_keys (actor_subject, operation);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires_at ON idempotency_keys (expires_at);

COMMIT;
