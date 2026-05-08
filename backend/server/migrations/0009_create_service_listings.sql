BEGIN;

-- Create service_listings table (Separate Table Design - eBay/Amazon Style)
CREATE TABLE IF NOT EXISTS service_listings (
    listing_id VARCHAR(64) PRIMARY KEY REFERENCES listings(listing_id) ON DELETE CASCADE,
    service_type VARCHAR(20) NOT NULL CHECK (service_type IN ('local', 'online')),
    hourly_rate DECIMAL(10,2),
    project_rate DECIMAL(10,2),
    availability JSONB,  -- Store schedule as JSON: [{"day": "Monday", "slots": ["09:00-12:00"]}]
    qualifications JSONB, -- Store as JSON array: ["Teaching License", "Math Degree"]
    service_radius_km INT, -- For local services (travel radius)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes for service_listings
CREATE INDEX IF NOT EXISTS idx_service_listings_service_type ON service_listings(service_type);
CREATE INDEX IF NOT EXISTS idx_service_listings_hourly_rate ON service_listings(hourly_rate);
CREATE INDEX IF NOT EXISTS idx_service_listings_service_radius ON service_listings(service_radius_km);

COMMIT;
