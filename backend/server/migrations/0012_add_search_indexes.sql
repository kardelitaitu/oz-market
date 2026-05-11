-- Migration: Optimize search indexes from benchmark query patterns
-- Date: 2026-05-11
-- Description: Add functional index for lower(city) queries,
--              add composite index for (listing_type, status, category),
--              ensure pg_trgm extension for GIN trigram search

-- -------------------------------------------------------------------
-- Part 1: Optimize location index for lower(city) query pattern
-- -------------------------------------------------------------------
-- The search query uses `lower(city) = lower($1)` which prevents a
-- standard b-tree index from being used for the city column.
-- Replace with a functional index on (country_code, LOWER(city)).
DROP INDEX IF EXISTS idx_listings_location;
CREATE INDEX IF NOT EXISTS idx_listings_location_func
  ON listings (country_code, LOWER(city));

COMMENT ON INDEX idx_listings_location_func
  IS 'Functional index supporting country_code + lower(city) lookups';

-- -------------------------------------------------------------------
-- Part 2: Composite index for common WHERE clause combinations
-- -------------------------------------------------------------------
-- Benchmark query patterns show most searches filter by listing_type,
-- status, and category together. A composite index covering all three
-- enables efficient index-only scans for these common patterns.
CREATE INDEX IF NOT EXISTS idx_listings_type_status_category
  ON listings (listing_type, status, category);

COMMENT ON INDEX idx_listings_type_status_category
  IS 'Composite index for common filtered search (type + status + category)';

-- -------------------------------------------------------------------
-- Part 3: Ensure pg_trgm extension exists for GIN trigram search
-- -------------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS pg_trgm;
