ALTER TABLE user_groups
    ADD COLUMN concurrent_limit INT NULL,
    ADD COLUMN concurrent_limit_mode VARCHAR(32) NOT NULL DEFAULT 'inherit';
