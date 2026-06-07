WITH configured_group AS (
    SELECT TRIM(BOTH '"' FROM value::text) AS group_id
    FROM public.system_configs
    WHERE key = 'default_user_group_id'
    LIMIT 1
),
group_candidates AS (
    SELECT group_id, 0 AS sort_rank, 0 AS priority, '' AS name
    FROM configured_group
    UNION ALL
    SELECT '00000000-0000-0000-0000-000000000001', 1, 0, ''
    UNION ALL
    SELECT id, 2, priority, name
    FROM public.user_groups
    WHERE visibility = 'public'
),
default_group AS (
    SELECT group_candidates.group_id
    FROM group_candidates
    JOIN public.user_groups ON user_groups.id = group_candidates.group_id
    ORDER BY group_candidates.sort_rank ASC,
             group_candidates.priority DESC,
             group_candidates.name ASC,
             group_candidates.group_id ASC
    LIMIT 1
)
UPDATE public.api_keys
SET group_id = (SELECT group_id FROM default_group)
WHERE group_id IS NULL
  AND is_standalone IS FALSE
  AND EXISTS (SELECT 1 FROM default_group);
