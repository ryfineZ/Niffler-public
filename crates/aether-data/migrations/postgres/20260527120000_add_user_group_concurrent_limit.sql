ALTER TABLE public.user_groups
    ADD COLUMN IF NOT EXISTS concurrent_limit integer;

ALTER TABLE public.user_groups
    ADD COLUMN IF NOT EXISTS concurrent_limit_mode text DEFAULT 'inherit' NOT NULL;

ALTER TABLE public.user_groups
    DROP CONSTRAINT IF EXISTS user_groups_concurrent_limit_mode_check;

ALTER TABLE public.user_groups
    ADD CONSTRAINT user_groups_concurrent_limit_mode_check
        CHECK (concurrent_limit_mode IN ('inherit', 'system', 'custom'));
