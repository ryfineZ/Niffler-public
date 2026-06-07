ALTER TABLE public.user_groups
    ADD COLUMN IF NOT EXISTS visibility text DEFAULT 'public' NOT NULL,
    ADD COLUMN IF NOT EXISTS sales_multiplier double precision DEFAULT 1 NOT NULL,
    ADD COLUMN IF NOT EXISTS model_sales_multipliers json;

ALTER TABLE public.user_groups
    DROP CONSTRAINT IF EXISTS user_groups_visibility_check;

ALTER TABLE public.user_groups
    ADD CONSTRAINT user_groups_visibility_check
        CHECK (visibility IN ('public', 'internal'));

ALTER TABLE public.user_groups
    DROP CONSTRAINT IF EXISTS user_groups_sales_multiplier_check;

ALTER TABLE public.user_groups
    ADD CONSTRAINT user_groups_sales_multiplier_check
        CHECK (sales_multiplier >= 0);

ALTER TABLE public.api_keys
    ADD COLUMN IF NOT EXISTS group_id character varying(64);

UPDATE public.api_keys
SET group_id = COALESCE(
    (
        SELECT TRIM(BOTH '"' FROM value::text)
        FROM public.system_configs
        WHERE key = 'default_user_group_id'
        LIMIT 1
    ),
    '00000000-0000-0000-0000-000000000001'
)
WHERE group_id IS NULL
  AND is_standalone IS FALSE;

CREATE INDEX IF NOT EXISTS api_keys_group_id_idx
    ON public.api_keys (group_id);

ALTER TABLE public.api_keys
    DROP CONSTRAINT IF EXISTS api_keys_group_id_fkey;

ALTER TABLE public.api_keys
    ADD CONSTRAINT api_keys_group_id_fkey
        FOREIGN KEY (group_id) REFERENCES public.user_groups(id) ON DELETE RESTRICT;
