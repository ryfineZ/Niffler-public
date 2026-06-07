UPDATE api_keys
JOIN system_configs AS config
  ON config.`key` = 'default_user_group_id'
JOIN user_groups AS default_group
  ON default_group.id = JSON_UNQUOTE(config.value)
SET api_keys.group_id = default_group.id
WHERE api_keys.group_id IS NULL
  AND api_keys.is_standalone = 0;

UPDATE api_keys
JOIN user_groups AS builtin_group
  ON builtin_group.id = '00000000-0000-0000-0000-000000000001'
SET api_keys.group_id = builtin_group.id
WHERE api_keys.group_id IS NULL
  AND api_keys.is_standalone = 0;

UPDATE api_keys
JOIN (
    SELECT id
    FROM user_groups
    WHERE visibility = 'public'
    ORDER BY priority DESC, name ASC, id ASC
    LIMIT 1
) AS fallback_group
SET api_keys.group_id = fallback_group.id
WHERE api_keys.group_id IS NULL
  AND api_keys.is_standalone = 0;
