-- Migration: Create user_accounts table for basic user management
-- Date: 2026-05-10
-- Description: Add minimal user account storage for registration/login

CREATE TABLE IF NOT EXISTS user_accounts (
    user_id TEXT PRIMARY KEY,
    username TEXT UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    phone_number TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    phone_verified BOOLEAN NOT NULL DEFAULT FALSE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_user_accounts_username ON user_accounts (username);
CREATE INDEX IF NOT EXISTS idx_user_accounts_email ON user_accounts (email);
CREATE INDEX IF NOT EXISTS idx_user_accounts_status ON user_accounts (status);

-- Comments for documentation
COMMENT ON TABLE user_accounts IS 'Basic user account information for authentication';
COMMENT ON COLUMN user_accounts.user_id IS 'Primary user identifier (UUID or similar)';
COMMENT ON COLUMN user_accounts.username IS 'Unique username for login';
COMMENT ON COLUMN user_accounts.email IS 'Unique email address';
COMMENT ON COLUMN user_accounts.password_hash IS 'bcrypt or argon2 password hash';
COMMENT ON COLUMN user_accounts.phone_number IS 'Optional phone number for verification';
COMMENT ON COLUMN user_accounts.email_verified IS 'Whether email has been verified';
COMMENT ON COLUMN user_accounts.phone_verified IS 'Whether phone has been verified';

-- Link seller_accounts to user_accounts (optional, for sellers who have accounts)
ALTER TABLE seller_accounts ADD COLUMN IF NOT EXISTS user_id TEXT REFERENCES user_accounts(user_id);

COMMENT ON COLUMN seller_accounts.user_id IS 'Optional link to user account for sellers with marketplace accounts';