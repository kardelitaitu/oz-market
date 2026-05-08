BEGIN;

-- Add listing_type column to listings table
ALTER TABLE listings ADD COLUMN IF NOT EXISTS listing_type VARCHAR(20) NOT NULL DEFAULT 'product';

-- Update existing rows to 'product' (safety check)
UPDATE listings SET listing_type = 'product' WHERE listing_type IS NULL;

-- Add check constraint
ALTER TABLE listings DROP CONSTRAINT IF EXISTS listings_listing_type_check;
ALTER TABLE listings ADD CONSTRAINT listings_listing_type_check 
    CHECK (listing_type IN ('product', 'service', 'property'));

-- Add index for listing_type
CREATE INDEX IF NOT EXISTS idx_listings_listing_type ON listings (listing_type);

COMMIT;
