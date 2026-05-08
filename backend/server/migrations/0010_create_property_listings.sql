BEGIN;

-- Create property_listings table (Separate Table Design - Zillow style)
CREATE TABLE IF NOT EXISTS property_listings (
    listing_id VARCHAR(64) PRIMARY KEY REFERENCES listings(listing_id) ON DELETE CASCADE,
    property_transaction_type VARCHAR(10) NOT NULL CHECK (property_transaction_type IN ('rent', 'sale')),
    property_sub_type VARCHAR(20) NOT NULL CHECK (property_sub_type IN ('building', 'house', 'apartment', 'land')),
    area_sqm DECIMAL(10,2),
    bedrooms INT,          -- For house/apartment
    bathrooms INT,         -- For house/apartment
    year_built INT,        -- For building/house/apartment
    lot_size_sqm DECIMAL(10,2), -- For land
    zoning VARCHAR(50)      -- For land (residential, commercial, agricultural)
);

-- Indexes for property_listings
CREATE INDEX IF NOT EXISTS idx_property_listings_transaction_type ON property_listings(property_transaction_type);
CREATE INDEX IF NOT EXISTS idx_property_listings_sub_type ON property_listings(property_sub_type);
CREATE INDEX IF NOT EXISTS idx_property_listings_bedrooms ON property_listings(bedrooms);
CREATE INDEX IF NOT EXISTS idx_property_listings_bathrooms ON property_listings(bathrooms);
CREATE INDEX IF NOT EXISTS idx_property_listings_area ON property_listings(area_sqm);
CREATE INDEX IF NOT EXISTS idx_property_listings_lot_size ON property_listings(lot_size_sqm);

COMMIT;
