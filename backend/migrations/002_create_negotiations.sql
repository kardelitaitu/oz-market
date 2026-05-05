-- Create negotiations table
CREATE TABLE IF NOT EXISTS negotiations (
    negotiation_id VARCHAR(255) PRIMARY KEY,
    version BIGINT NOT NULL DEFAULT 1,
    status VARCHAR(50) NOT NULL DEFAULT 'open',
    listing_id VARCHAR(255) NOT NULL,
    buyer_agent_id VARCHAR(255) NOT NULL,
    seller_account_id VARCHAR(255) NOT NULL,
    current_offer JSONB,
    reveal_request JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_negotiations_status ON negotiations(status);
CREATE INDEX idx_negotiations_listing ON negotiations(listing_id);
CREATE INDEX idx_negotiations_buyer ON negotiations(buyer_agent_id);
CREATE INDEX idx_negotiations_seller ON negotiations(seller_account_id);
