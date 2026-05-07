BEGIN;

-- Drop the foreign key constraint on reservation_leases.negotiation_id
-- since we don't use the negotiations table directly
ALTER TABLE reservation_leases DROP CONSTRAINT IF EXISTS reservation_leases_negotiation_id_fkey;

-- Make negotiation_id nullable since we may not always have a negotiation
ALTER TABLE reservation_leases ALTER COLUMN negotiation_id DROP NOT NULL;

COMMIT;
