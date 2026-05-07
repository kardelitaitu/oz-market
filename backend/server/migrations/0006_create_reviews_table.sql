-- Migration: Create reviews table and add rating calculation
-- Date: 2026-05-08
-- Description: Add reviews system with automatic seller_rating calculation

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

-- Function to recalculate seller_rating
CREATE OR REPLACE FUNCTION update_seller_rating()
RETURNS TRIGGER AS $$
BEGIN
    -- Update seller_rating for the affected seller
    UPDATE seller_accounts 
    SET seller_rating = (
        SELECT AVG(rating)::DECIMAL(3,2)
        FROM reviews 
        WHERE seller_account_id = COALESCE(NEW.seller_account_id, OLD.seller_account_id)
        AND status = 'approved'
    )
    WHERE seller_account_id = COALESCE(NEW.seller_account_id, OLD.seller_account_id);
    
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger for INSERT
CREATE OR REPLACE TRIGGER trigger_reviews_insert
AFTER INSERT ON reviews
FOR EACH ROW
EXECUTE FUNCTION update_seller_rating();

-- Trigger for UPDATE (status changes)
CREATE OR REPLACE TRIGGER trigger_reviews_update
AFTER UPDATE ON reviews
FOR EACH ROW
WHEN (OLD.status IS DISTINCT FROM NEW.status OR OLD.rating IS DISTINCT FROM NEW.rating)
EXECUTE FUNCTION update_seller_rating();

-- Trigger for DELETE
CREATE OR REPLACE TRIGGER trigger_reviews_delete
AFTER DELETE ON reviews
FOR EACH ROW
EXECUTE FUNCTION update_seller_rating();

-- Add comment
COMMENT ON TABLE reviews IS 'Buyer reviews for listings/sellers';
COMMENT ON COLUMN reviews.rating IS '1-5 star rating';
COMMENT ON COLUMN reviews.status IS 'pending=awaiting moderation, approved=counts toward rating, rejected=does not count';
