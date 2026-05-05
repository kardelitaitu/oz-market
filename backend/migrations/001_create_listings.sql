-- Create listings table
CREATE TABLE IF NOT EXISTS listings (
    listing_id VARCHAR(255) PRIMARY KEY,
    version BIGINT NOT NULL DEFAULT 1,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    schema_version VARCHAR(50) NOT NULL DEFAULT '1.0',
    owner_id VARCHAR(255) NOT NULL,
    category TEXT NOT NULL,
    product_name TEXT NOT NULL,
    condition TEXT NOT NULL,
    price_currency VARCHAR(10) NOT NULL,
    price_amount DOUBLE PRECISION NOT NULL,
    country_code VARCHAR(10) NOT NULL,
    country_name TEXT NOT NULL,
    city TEXT NOT NULL,
    picture_urls TEXT[] DEFAULT '{}',
    description TEXT NOT NULL,
    attributes JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_listings_status ON listings(status);
CREATE INDEX idx_listings_category ON listings(category);
CREATE INDEX idx_listings_price ON listings(price_amount);
CREATE INDEX idx_listings_location ON listings(country_code, city);
CREATE INDEX idx_listings_search ON listings USING GIN (
    to_tsvector('english', product_name || ' ' || description)
);
