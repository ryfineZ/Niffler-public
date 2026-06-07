CREATE TABLE IF NOT EXISTS public.entitlement_usage_windows (
    id character varying(64) PRIMARY KEY,
    user_entitlement_id character varying(64) NOT NULL REFERENCES public.user_plan_entitlements(id) ON DELETE CASCADE,
    user_id character varying(64) NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    window_scope character varying(32) NOT NULL,
    window_key character varying(64) NOT NULL,
    window_started_at timestamp with time zone NOT NULL,
    window_ends_at timestamp with time zone NOT NULL,
    used_usd numeric(20,8) NOT NULL DEFAULT 0,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_entitlement_usage_window UNIQUE (user_entitlement_id, window_scope)
);

CREATE INDEX IF NOT EXISTS idx_entitlement_usage_windows_user_scope
  ON public.entitlement_usage_windows USING btree (user_id, window_scope, window_ends_at);
