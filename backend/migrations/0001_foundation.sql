-- Migration 0001: Foundation — extensions and shared utilities
-- This migration enables the extensions needed by all subsequent migrations.

-- Enable UUID generation functions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable case-insensitive text (used for email/username lookups)
CREATE EXTENSION IF NOT EXISTS "citext";

-- Utility function: automatically update `updated_at` timestamp
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
