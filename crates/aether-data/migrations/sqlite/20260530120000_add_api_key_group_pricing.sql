ALTER TABLE user_groups ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public';
ALTER TABLE user_groups ADD COLUMN sales_multiplier REAL NOT NULL DEFAULT 1;
ALTER TABLE user_groups ADD COLUMN model_sales_multipliers TEXT;

ALTER TABLE api_keys ADD COLUMN group_id TEXT;

UPDATE api_keys
SET group_id = COALESCE(
    (
        SELECT json_extract(value, '$')
        FROM system_configs
        WHERE key = 'default_user_group_id'
        LIMIT 1
    ),
    '00000000-0000-0000-0000-000000000001'
)
WHERE group_id IS NULL
  AND is_standalone = 0;

CREATE INDEX api_keys_group_id_idx ON api_keys (group_id);
