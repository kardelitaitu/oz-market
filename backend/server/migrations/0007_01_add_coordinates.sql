-- Migration: Add geolocation columns to listings table
-- Phase D: Geolocation Search ("Near Me")

ALTER TABLE listings 
ADD COLUMN IF NOT EXISTS latitude DECIMAL(10,8),
ADD COLUMN IF NOT EXISTS longitude DECIMAL(11,8),
ADD COLUMN IF NOT EXISTS geolocation_opt_out BOOLEAN DEFAULT FALSE;

-- Create index for distance queries (optional, for performance)
CREATE INDEX IF NOT EXISTS idx_listings_coordinates 
ON listings (latitude, longitude) 
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- Comments
COMMENT ON COLUMN listings.latitude IS 'Latitude for geolocation (decimal degrees)';
COMMENT ON COLUMN listings.longitude IS 'Longitude for geolocation (decimal degrees)';
COMMENT ON COLUMN listings.geolocation_opt_out IS 'Seller opted out of geolocation search';
