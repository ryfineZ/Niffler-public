ALTER TABLE user_groups ADD COLUMN concurrent_limit INTEGER;
ALTER TABLE user_groups ADD COLUMN concurrent_limit_mode TEXT NOT NULL DEFAULT 'inherit';
