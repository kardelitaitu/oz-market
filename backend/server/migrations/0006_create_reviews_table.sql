-- Migration: Create reviews table
-- Date: 2026-05-08
-- Description: Add reviews table (triggers must be applied manually via 0006_triggers.sql)

CREATE TABLE IF NOT EXISTS reviews (
    review_id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL REFERENCES listings (listing_id) ON DELETE CASCADE,
    seller_account_id TEXT NOT NULL REFERENCES seller_accounts (seller_account_id) ON DELETE CASCADE,
    reviewer_id TEXT NOT NULL, -- buyer agent ID or user ID
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title TEXT NOT NULL CHECK (length(title) >= 3 AND length(title) <= 200),
    body TEXT CHECK (length(body) <= 2000),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_reviews_listing_id ON reviews (listing_id);
CREATE INDEX IF NOT EXISTS idx_reviews_seller_account_id ON reviews (seller_account_id);
CREATE INDEX IF NOT EXISTS idx_reviews_status ON reviews (status);

-- Add comment
COMMENT ON TABLE reviews IS 'Buyer reviews for listings/sellers';
COMMENT ON COLUMN reviews.rating IS '1-5 star rating';
COMMENT ON COLUMN reviews.status IS 'pending=awaiting moderation, approved=counts toward rating, rejected=does not count';
