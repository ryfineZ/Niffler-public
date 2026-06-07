ALTER TABLE user_groups
    ADD COLUMN visibility VARCHAR(32) NOT NULL DEFAULT 'public',
    ADD COLUMN sales_multiplier DOUBLE NOT NULL DEFAULT 1,
    ADD COLUMN model_sales_multipliers JSON NULL;

ALTER TABLE api_keys
    ADD COLUMN group_id VARCHAR(64) NULL;

UPDATE api_keys
SET group_id = COALESCE(
    (
        SELECT JSON_UNQUOTE(value)
        FROM system_configs
        WHERE `key` = 'default_user_group_id'
        LIMIT 1
    ),
    '00000000-0000-0000-0000-000000000001'
)
WHERE group_id IS NULL
  AND is_standalone = 0;

CREATE INDEX api_keys_group_id_idx ON api_keys (group_id);
