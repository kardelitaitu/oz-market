-- Migration: Add marketplace fields to listings table
-- Date: 2026-05-07
-- Description: Add sku, quantity, shipping_info, condition_details, seller_notes fields

ALTER TABLE listings ADD COLUMN IF NOT EXISTS sku VARCHAR(100);
ALTER TABLE listings ADD COLUMN IF NOT EXISTS quantity INTEGER DEFAULT 1;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS shipping_info JSONB;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS condition_details TEXT;
ALTER TABLE listings ADD COLUMN IF NOT EXISTS seller_notes TEXT;

-- Add comments for documentation
COMMENT ON COLUMN listings.sku IS 'Seller''s inventory SKU for tracking';
COMMENT ON COLUMN listings.quantity IS 'Number of identical items available (default: 1)';
COMMENT ON COLUMN listings.shipping_info IS 'JSONB: {local_pickup, shipping_available, shipping_cost, shipping_regions}';
COMMENT ON COLUMN listings.condition_details IS 'Granular condition description (e.g., "like new")';
COMMENT ON COLUMN listings.seller_notes IS 'Additional notes for buyers';
