-- Migration: Add display_name and seller_rating to seller_accounts
-- Date: 2026-05-08
-- Description: Add fields for better seller display and rating

ALTER TABLE seller_accounts ADD COLUMN IF NOT EXISTS display_name VARCHAR(200);
ALTER TABLE seller_accounts ADD COLUMN IF NOT EXISTS seller_rating DECIMAL(3,2) CHECK (seller_rating >= 0.0 AND seller_rating <= 5.0);

-- Add comments for documentation
COMMENT ON COLUMN seller_accounts.display_name IS 'Human-readable seller display name';
COMMENT ON COLUMN seller_accounts.seller_rating IS 'Average seller rating (0.00-5.00), calculated from reviews';
