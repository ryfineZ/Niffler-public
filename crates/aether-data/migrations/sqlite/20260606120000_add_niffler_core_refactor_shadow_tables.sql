CREATE TABLE IF NOT EXISTS niffler_upstream_services (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    service_kind TEXT NOT NULL,
    default_api_format TEXT,
    base_url TEXT,
    cost_multiplier REAL NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1,
    config TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK (cost_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_services_kind_active
    ON niffler_upstream_services (service_kind, is_active);

CREATE TABLE IF NOT EXISTS niffler_upstream_accounts (
    id TEXT PRIMARY KEY,
    upstream_service_id TEXT NOT NULL REFERENCES niffler_upstream_services (id),
    display_name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    auth_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'available',
    cost_multiplier REAL NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    cooldown_until_unix_ms INTEGER,
    last_tested_at_unix_ms INTEGER,
    last_test_error TEXT,
    config TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK (status IN ('available', 'disabled', 'invalid', 'quota_exhausted', 'cooling_down')),
    CHECK (cost_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_accounts_service_status
    ON niffler_upstream_accounts (upstream_service_id, status, priority);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_accounts_email
    ON niffler_upstream_accounts (email);

CREATE TABLE IF NOT EXISTS niffler_product_plans (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    is_public INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    sales_multiplier REAL NOT NULL DEFAULT 1,
    description TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK (sales_multiplier >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_product_plans_public_active
    ON niffler_product_plans (is_public, is_active);

CREATE TABLE IF NOT EXISTS niffler_product_plan_models (
    id TEXT PRIMARY KEY,
    product_plan_id TEXT NOT NULL REFERENCES niffler_product_plans (id),
    model_name TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    sales_multiplier_override REAL,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE (product_plan_id, model_name),
    CHECK (sales_multiplier_override IS NULL OR sales_multiplier_override >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_product_plan_models_model
    ON niffler_product_plan_models (model_name, is_enabled);

CREATE TABLE IF NOT EXISTS niffler_model_base_prices (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL,
    input_price_per_million REAL NOT NULL DEFAULT 0,
    output_price_per_million REAL NOT NULL DEFAULT 0,
    cache_write_price_per_million REAL,
    cache_read_price_per_million REAL,
    source TEXT NOT NULL,
    effective_from_unix_ms INTEGER NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK (
        input_price_per_million >= 0
        AND output_price_per_million >= 0
        AND (cache_write_price_per_million IS NULL OR cache_write_price_per_million >= 0)
        AND (cache_read_price_per_million IS NULL OR cache_read_price_per_million >= 0)
    )
);

CREATE INDEX IF NOT EXISTS idx_niffler_model_base_prices_model_effective
    ON niffler_model_base_prices (model_name, effective_from_unix_ms DESC);

CREATE TABLE IF NOT EXISTS niffler_upstream_model_prices (
    id TEXT PRIMARY KEY,
    upstream_service_id TEXT NOT NULL REFERENCES niffler_upstream_services (id),
    model_name TEXT NOT NULL,
    upstream_input_price_per_million REAL,
    upstream_output_price_per_million REAL,
    upstream_cache_write_price_per_million REAL,
    upstream_cache_read_price_per_million REAL,
    price_source_preference TEXT NOT NULL DEFAULT 'official',
    source TEXT,
    synced_at_unix_ms INTEGER,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE (upstream_service_id, model_name),
    CHECK (price_source_preference IN ('official', 'upstream'))
);

CREATE TABLE IF NOT EXISTS niffler_account_model_capabilities (
    id TEXT PRIMARY KEY,
    upstream_service_id TEXT NOT NULL REFERENCES niffler_upstream_services (id),
    upstream_account_id TEXT NOT NULL REFERENCES niffler_upstream_accounts (id),
    model_name TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    source TEXT,
    last_checked_at_unix_ms INTEGER,
    last_error TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE (upstream_account_id, model_name)
);

CREATE INDEX IF NOT EXISTS idx_niffler_account_model_capabilities_model
    ON niffler_account_model_capabilities (model_name, is_enabled);

CREATE TABLE IF NOT EXISTS niffler_route_attempts (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    upstream_service_id TEXT,
    upstream_account_id TEXT,
    product_plan_id TEXT,
    model_name TEXT NOT NULL,
    attempt_index INTEGER NOT NULL,
    status TEXT NOT NULL,
    skip_reason TEXT,
    upstream_status_code INTEGER,
    latency_ms INTEGER,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_niffler_route_attempts_request
    ON niffler_route_attempts (request_id, attempt_index);

CREATE INDEX IF NOT EXISTS idx_niffler_route_attempts_account
    ON niffler_route_attempts (upstream_account_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS niffler_error_return_settings (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    upstream_service_id TEXT,
    match_status_code INTEGER,
    match_text TEXT,
    handling_step TEXT,
    response_mode TEXT NOT NULL DEFAULT 'replace',
    user_message TEXT NOT NULL,
    account_protection_action TEXT NOT NULL DEFAULT 'record_only',
    pause_duration TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK (scope IN ('platform', 'upstream')),
    CHECK (response_mode IN ('replace', 'append', 'redact')),
    CHECK (account_protection_action IN ('record_only', 'pause_scheduling', 'disable_account')),
    CHECK (pause_duration IS NULL OR pause_duration IN ('ten_minutes', 'one_hour', 'twenty_four_hours', 'manual_restore'))
);

CREATE INDEX IF NOT EXISTS idx_niffler_error_return_settings_scope_active
    ON niffler_error_return_settings (scope, is_active);

CREATE INDEX IF NOT EXISTS idx_niffler_error_return_settings_upstream
    ON niffler_error_return_settings (upstream_service_id, is_active);

CREATE TABLE IF NOT EXISTS niffler_account_risk_events (
    id TEXT PRIMARY KEY,
    upstream_service_id TEXT,
    upstream_account_id TEXT NOT NULL,
    request_id TEXT,
    user_id TEXT,
    api_key_id TEXT,
    model_name TEXT,
    rule_id TEXT,
    matched_text TEXT,
    upstream_status_code INTEGER,
    action TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_niffler_account_risk_events_account_time
    ON niffler_account_risk_events (upstream_account_id, created_at_unix_ms DESC);

CREATE INDEX IF NOT EXISTS idx_niffler_account_risk_events_request
    ON niffler_account_risk_events (request_id);

CREATE TABLE IF NOT EXISTS niffler_api_key_pauses (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    user_message TEXT NOT NULL,
    paused_until_unix_ms INTEGER,
    manual_restore_required INTEGER NOT NULL DEFAULT 0,
    created_at_unix_ms INTEGER NOT NULL,
    restored_at_unix_ms INTEGER,
    restored_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_niffler_api_key_pauses_key_active
    ON niffler_api_key_pauses (api_key_id, restored_at_unix_ms, paused_until_unix_ms);
