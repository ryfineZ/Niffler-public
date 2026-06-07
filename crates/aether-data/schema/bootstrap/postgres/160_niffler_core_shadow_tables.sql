CREATE TABLE IF NOT EXISTS public.niffler_upstream_services (
    id character varying(36) PRIMARY KEY,
    display_name character varying(200) NOT NULL,
    service_kind character varying(64) NOT NULL,
    default_api_format character varying(64),
    base_url text,
    cost_multiplier double precision DEFAULT 1 NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    config jsonb,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_upstream_services_cost_multiplier
        CHECK (cost_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_services_kind_active
    ON public.niffler_upstream_services (service_kind, is_active);

CREATE TABLE IF NOT EXISTS public.niffler_upstream_accounts (
    id character varying(36) PRIMARY KEY,
    upstream_service_id character varying(36) NOT NULL
        REFERENCES public.niffler_upstream_services (id),
    display_name character varying(200) NOT NULL,
    email character varying(320),
    phone character varying(64),
    auth_kind character varying(64) NOT NULL,
    status character varying(32) DEFAULT 'available' NOT NULL,
    cost_multiplier double precision DEFAULT 1 NOT NULL,
    priority integer DEFAULT 0 NOT NULL,
    cooldown_until_unix_ms bigint,
    last_tested_at_unix_ms bigint,
    last_test_error text,
    config jsonb,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_upstream_accounts_status
        CHECK (status IN ('available', 'disabled', 'invalid', 'quota_exhausted', 'cooling_down')),
    CONSTRAINT ck_niffler_upstream_accounts_cost_multiplier
        CHECK (cost_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_accounts_service_status
    ON public.niffler_upstream_accounts (upstream_service_id, status, priority);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_accounts_email
    ON public.niffler_upstream_accounts (email);

CREATE TABLE IF NOT EXISTS public.niffler_product_plans (
    id character varying(36) PRIMARY KEY,
    display_name character varying(200) NOT NULL,
    is_public boolean DEFAULT false NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    sales_multiplier double precision DEFAULT 1 NOT NULL,
    description text,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_product_plans_sales_multiplier
        CHECK (sales_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_product_plans_public_active
    ON public.niffler_product_plans (is_public, is_active);

CREATE TABLE IF NOT EXISTS public.niffler_product_plan_models (
    id character varying(36) PRIMARY KEY,
    product_plan_id character varying(36) NOT NULL
        REFERENCES public.niffler_product_plans (id),
    model_name character varying(200) NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    sales_multiplier_override double precision,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_product_plan_models_sales_override
        CHECK (sales_multiplier_override IS NULL OR sales_multiplier_override >= 0),
    CONSTRAINT uq_niffler_product_plan_models_plan_model
        UNIQUE (product_plan_id, model_name)
);

CREATE INDEX IF NOT EXISTS idx_niffler_product_plan_models_model
    ON public.niffler_product_plan_models (model_name, is_enabled);

CREATE TABLE IF NOT EXISTS public.niffler_model_base_prices (
    id character varying(36) PRIMARY KEY,
    model_name character varying(200) NOT NULL,
    input_price_per_million numeric(20, 8) DEFAULT 0 NOT NULL,
    output_price_per_million numeric(20, 8) DEFAULT 0 NOT NULL,
    cache_write_price_per_million numeric(20, 8),
    cache_read_price_per_million numeric(20, 8),
    source character varying(64) NOT NULL,
    effective_from_unix_ms bigint NOT NULL,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_model_base_prices_non_negative
        CHECK (
            input_price_per_million >= 0
            AND output_price_per_million >= 0
            AND (cache_write_price_per_million IS NULL OR cache_write_price_per_million >= 0)
            AND (cache_read_price_per_million IS NULL OR cache_read_price_per_million >= 0)
        )
);

CREATE INDEX IF NOT EXISTS idx_niffler_model_base_prices_model_effective
    ON public.niffler_model_base_prices (model_name, effective_from_unix_ms DESC);

CREATE TABLE IF NOT EXISTS public.niffler_upstream_model_prices (
    id character varying(36) PRIMARY KEY,
    upstream_service_id character varying(36) NOT NULL
        REFERENCES public.niffler_upstream_services (id),
    model_name character varying(200) NOT NULL,
    upstream_input_price_per_million numeric(20, 8),
    upstream_output_price_per_million numeric(20, 8),
    upstream_cache_write_price_per_million numeric(20, 8),
    upstream_cache_read_price_per_million numeric(20, 8),
    price_source_preference character varying(32) DEFAULT 'official' NOT NULL,
    source character varying(64),
    synced_at_unix_ms bigint,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_upstream_model_prices_preference
        CHECK (price_source_preference IN ('official', 'upstream')),
    CONSTRAINT uq_niffler_upstream_model_prices_service_model
        UNIQUE (upstream_service_id, model_name)
);

CREATE TABLE IF NOT EXISTS public.niffler_account_model_capabilities (
    id character varying(36) PRIMARY KEY,
    upstream_service_id character varying(36) NOT NULL
        REFERENCES public.niffler_upstream_services (id),
    upstream_account_id character varying(36) NOT NULL
        REFERENCES public.niffler_upstream_accounts (id),
    model_name character varying(200) NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    source character varying(64),
    last_checked_at_unix_ms bigint,
    last_error text,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT uq_niffler_account_model_capabilities_account_model
        UNIQUE (upstream_account_id, model_name)
);

CREATE INDEX IF NOT EXISTS idx_niffler_account_model_capabilities_model
    ON public.niffler_account_model_capabilities (model_name, is_enabled);

CREATE TABLE IF NOT EXISTS public.niffler_route_attempts (
    id character varying(36) PRIMARY KEY,
    request_id character varying(100) NOT NULL,
    upstream_service_id character varying(36),
    upstream_account_id character varying(36),
    product_plan_id character varying(36),
    model_name character varying(200) NOT NULL,
    attempt_index integer NOT NULL,
    status character varying(32) NOT NULL,
    skip_reason text,
    upstream_status_code integer,
    latency_ms bigint,
    created_at_unix_ms bigint NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_niffler_route_attempts_request
    ON public.niffler_route_attempts (request_id, attempt_index);

CREATE INDEX IF NOT EXISTS idx_niffler_route_attempts_account
    ON public.niffler_route_attempts (upstream_account_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS public.niffler_error_return_settings (
    id character varying(36) PRIMARY KEY,
    scope character varying(32) NOT NULL,
    upstream_service_id character varying(36),
    match_status_code integer,
    match_text text,
    handling_step character varying(64),
    response_mode character varying(32) DEFAULT 'replace' NOT NULL,
    user_message text NOT NULL,
    account_protection_action character varying(32) DEFAULT 'record_only' NOT NULL,
    pause_duration character varying(32),
    is_active boolean DEFAULT true NOT NULL,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_error_return_settings_scope
        CHECK (scope IN ('platform', 'upstream')),
    CONSTRAINT ck_niffler_error_return_settings_response_mode
        CHECK (response_mode IN ('replace', 'append', 'redact')),
    CONSTRAINT ck_niffler_error_return_settings_action
        CHECK (account_protection_action IN ('record_only', 'pause_scheduling', 'disable_account')),
    CONSTRAINT ck_niffler_error_return_settings_pause_duration
        CHECK (pause_duration IS NULL OR pause_duration IN ('ten_minutes', 'one_hour', 'twenty_four_hours', 'manual_restore'))
);

CREATE INDEX IF NOT EXISTS idx_niffler_error_return_settings_scope_active
    ON public.niffler_error_return_settings (scope, is_active);

CREATE INDEX IF NOT EXISTS idx_niffler_error_return_settings_upstream
    ON public.niffler_error_return_settings (upstream_service_id, is_active);

CREATE TABLE IF NOT EXISTS public.niffler_account_risk_events (
    id character varying(36) PRIMARY KEY,
    upstream_service_id character varying(36),
    upstream_account_id character varying(36) NOT NULL,
    request_id character varying(100),
    user_id character varying(36),
    api_key_id character varying(36),
    model_name character varying(200),
    rule_id character varying(36),
    matched_text text,
    upstream_status_code integer,
    action character varying(32) NOT NULL,
    created_at_unix_ms bigint NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_niffler_account_risk_events_account_time
    ON public.niffler_account_risk_events (upstream_account_id, created_at_unix_ms DESC);

CREATE INDEX IF NOT EXISTS idx_niffler_account_risk_events_request
    ON public.niffler_account_risk_events (request_id);

CREATE TABLE IF NOT EXISTS public.niffler_api_key_pauses (
    id character varying(36) PRIMARY KEY,
    api_key_id character varying(36) NOT NULL,
    reason_code character varying(64) NOT NULL,
    user_message text NOT NULL,
    paused_until_unix_ms bigint,
    manual_restore_required boolean DEFAULT false NOT NULL,
    created_at_unix_ms bigint NOT NULL,
    restored_at_unix_ms bigint,
    restored_by character varying(36)
);

CREATE INDEX IF NOT EXISTS idx_niffler_api_key_pauses_key_active
    ON public.niffler_api_key_pauses (api_key_id, restored_at_unix_ms, paused_until_unix_ms);
