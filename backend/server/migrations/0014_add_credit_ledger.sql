BEGIN;

CREATE TABLE IF NOT EXISTS agent_balances (
    agent_id TEXT PRIMARY KEY,
    balance_credits NUMERIC(20, 4) NOT NULL DEFAULT 0.0000,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_positive_balance CHECK (balance_credits >= 0.0000)
);

COMMENT ON TABLE agent_balances IS 'Tracks current credit balance per agent. agent_id logically references agent_credentials.subject.';
COMMENT ON COLUMN agent_balances.agent_id IS 'Unique agent identifier matching agent_credentials.subject';
COMMENT ON COLUMN agent_balances.balance_credits IS 'Current credit balance with 4-decimal precision, never negative';
COMMENT ON COLUMN agent_balances.updated_at IS 'Timestamp of last balance change';

CREATE TABLE IF NOT EXISTS credit_transactions (
    id UUID PRIMARY KEY,
    agent_id TEXT NOT NULL,
    amount NUMERIC(20, 4) NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    idempotency_key VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_tx_type CHECK (transaction_type IN ('deposit', 'spend', 'refund', 'adjustment'))
);

COMMENT ON TABLE credit_transactions IS 'Immutable audit log of every credit movement';
COMMENT ON COLUMN credit_transactions.id IS 'Unique transaction identifier (UUID v4)';
COMMENT ON COLUMN credit_transactions.agent_id IS 'Agent whose balance changed';
COMMENT ON COLUMN credit_transactions.amount IS 'Delta applied to balance (positive = credit, negative = debit)';
COMMENT ON COLUMN credit_transactions.transaction_type IS 'One of: deposit, spend, refund, adjustment';
COMMENT ON COLUMN credit_transactions.idempotency_key IS 'Client-supplied key to prevent duplicate processing';
COMMENT ON COLUMN credit_transactions.created_at IS 'Transaction timestamp';

CREATE INDEX IF NOT EXISTS idx_credit_transactions_agent_id ON credit_transactions(agent_id);
CREATE INDEX IF NOT EXISTS idx_credit_transactions_created_at ON credit_transactions(created_at DESC);

COMMIT;
