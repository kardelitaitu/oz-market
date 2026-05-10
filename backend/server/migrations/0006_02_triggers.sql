-- Run this in psql to add automatic seller_rating calculation
-- psql "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable" -f this_file.sql

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

-- Done! Now reviews will automatically update seller_rating
