-- Update a subset of listings with random coordinates (around New York)
-- Also set some listings to opt out

-- First, add coordinates to ~10% of listings (about 10,000)
UPDATE listings
SET latitude = 40.7128 + (random() - 0.5) * 0.5,
    longitude = -74.0060 + (random() - 0.5) * 0.5
WHERE listing_id IN (
    SELECT listing_id FROM listings
    WHERE latitude IS NULL
    LIMIT 10000
);

-- Set about 20% of those with coordinates to opt out
UPDATE listings
SET geolocation_opt_out = true
WHERE listing_id IN (
    SELECT listing_id FROM listings
    WHERE latitude IS NOT NULL
    ORDER BY random()
    LIMIT 2000
);

-- Show statistics
SELECT 
    COUNT(*) as total,
    COUNT(latitude) as with_coords,
    COUNT(*) FILTER (WHERE geolocation_opt_out = true) as opted_out,
    COUNT(*) FILTER (WHERE latitude IS NOT NULL AND (geolocation_opt_out IS NULL OR geolocation_opt_out = false)) as available_for_near_me
FROM listings;
