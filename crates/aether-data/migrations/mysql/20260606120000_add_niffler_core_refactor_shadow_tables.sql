CREATE TABLE IF NOT EXISTS niffler_upstream_services (
    id VARCHAR(36) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    service_kind VARCHAR(64) NOT NULL,
    default_api_format VARCHAR(64),
    base_url TEXT,
    cost_multiplier DOUBLE NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    config JSON,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_upstream_services_kind_active (service_kind, is_active),
    CHECK (cost_multiplier >= 0)
);

CREATE TABLE IF NOT EXISTS niffler_upstream_accounts (
    id VARCHAR(36) NOT NULL,
    upstream_service_id VARCHAR(36) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    email VARCHAR(320),
    phone VARCHAR(64),
    auth_kind VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'available',
    cost_multiplier DOUBLE NOT NULL DEFAULT 1,
    priority INT NOT NULL DEFAULT 0,
    cooldown_until_unix_ms BIGINT,
    last_tested_at_unix_ms BIGINT,
    last_test_error TEXT,
    config JSON,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_upstream_accounts_service_status (upstream_service_id, status, priority),
    INDEX idx_niffler_upstream_accounts_email (email),
    CONSTRAINT fk_niffler_upstream_accounts_service
        FOREIGN KEY (upstream_service_id) REFERENCES niffler_upstream_services (id),
    CHECK (status IN ('available', 'disabled', 'invalid', 'quota_exhausted', 'cooling_down')),
    CHECK (cost_multiplier >= 0)
);

CREATE TABLE IF NOT EXISTS niffler_product_plans (
    id VARCHAR(36) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    sales_multiplier DOUBLE NOT NULL DEFAULT 1,
    description TEXT,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_product_plans_public_active (is_public, is_active),
    CHECK (sales_multiplier >= 0)
);

CREATE TABLE IF NOT EXISTS niffler_product_plan_models (
    id VARCHAR(36) NOT NULL,
    product_plan_id VARCHAR(36) NOT NULL,
    model_name VARCHAR(200) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    sales_multiplier_override DOUBLE,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_product_plan_models_plan_model (product_plan_id, model_name),
    INDEX idx_niffler_product_plan_models_model (model_name, is_enabled),
    CONSTRAINT fk_niffler_product_plan_models_plan
        FOREIGN KEY (product_plan_id) REFERENCES niffler_product_plans (id),
    CHECK (sales_multiplier_override IS NULL OR sales_multiplier_override >= 0)
);

CREATE TABLE IF NOT EXISTS niffler_model_base_prices (
    id VARCHAR(36) NOT NULL,
    model_name VARCHAR(200) NOT NULL,
    input_price_per_million DECIMAL(20, 8) NOT NULL DEFAULT 0,
    output_price_per_million DECIMAL(20, 8) NOT NULL DEFAULT 0,
    cache_write_price_per_million DECIMAL(20, 8),
    cache_read_price_per_million DECIMAL(20, 8),
    source VARCHAR(64) NOT NULL,
    effective_from_unix_ms BIGINT NOT NULL,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_model_base_prices_model_effective (model_name, effective_from_unix_ms),
    CHECK (
        input_price_per_million >= 0
        AND output_price_per_million >= 0
        AND (cache_write_price_per_million IS NULL OR cache_write_price_per_million >= 0)
        AND (cache_read_price_per_million IS NULL OR cache_read_price_per_million >= 0)
    )
);

CREATE TABLE IF NOT EXISTS niffler_upstream_model_prices (
    id VARCHAR(36) NOT NULL,
    upstream_service_id VARCHAR(36) NOT NULL,
    model_name VARCHAR(200) NOT NULL,
    upstream_input_price_per_million DECIMAL(20, 8),
    upstream_output_price_per_million DECIMAL(20, 8),
    upstream_cache_write_price_per_million DECIMAL(20, 8),
    upstream_cache_read_price_per_million DECIMAL(20, 8),
    price_source_preference VARCHAR(32) NOT NULL DEFAULT 'official',
    source VARCHAR(64),
    synced_at_unix_ms BIGINT,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_upstream_model_prices_service_model (upstream_service_id, model_name),
    CONSTRAINT fk_niffler_upstream_model_prices_service
        FOREIGN KEY (upstream_service_id) REFERENCES niffler_upstream_services (id),
    CHECK (price_source_preference IN ('official', 'upstream'))
);

CREATE TABLE IF NOT EXISTS niffler_account_model_capabilities (
    id VARCHAR(36) NOT NULL,
    upstream_service_id VARCHAR(36) NOT NULL,
    upstream_account_id VARCHAR(36) NOT NULL,
    model_name VARCHAR(200) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    source VARCHAR(64),
    last_checked_at_unix_ms BIGINT,
    last_error TEXT,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_account_model_capabilities_account_model (upstream_account_id, model_name),
    INDEX idx_niffler_account_model_capabilities_model (model_name, is_enabled),
    CONSTRAINT fk_niffler_account_model_capabilities_service
        FOREIGN KEY (upstream_service_id) REFERENCES niffler_upstream_services (id),
    CONSTRAINT fk_niffler_account_model_capabilities_account
        FOREIGN KEY (upstream_account_id) REFERENCES niffler_upstream_accounts (id)
);

CREATE TABLE IF NOT EXISTS niffler_route_attempts (
    id VARCHAR(36) NOT NULL,
    request_id VARCHAR(100) NOT NULL,
    upstream_service_id VARCHAR(36),
    upstream_account_id VARCHAR(36),
    product_plan_id VARCHAR(36),
    model_name VARCHAR(200) NOT NULL,
    attempt_index INT NOT NULL,
    status VARCHAR(32) NOT NULL,
    skip_reason TEXT,
    upstream_status_code INT,
    latency_ms BIGINT,
    created_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_route_attempts_request (request_id, attempt_index),
    INDEX idx_niffler_route_attempts_account (upstream_account_id, created_at_unix_ms)
);

CREATE TABLE IF NOT EXISTS niffler_error_return_settings (
    id VARCHAR(36) NOT NULL,
    scope VARCHAR(32) NOT NULL,
    upstream_service_id VARCHAR(36),
    match_status_code INT,
    match_text TEXT,
    handling_step VARCHAR(64),
    response_mode VARCHAR(32) NOT NULL DEFAULT 'replace',
    user_message TEXT NOT NULL,
    account_protection_action VARCHAR(32) NOT NULL DEFAULT 'record_only',
    pause_duration VARCHAR(32),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_error_return_settings_scope_active (scope, is_active),
    INDEX idx_niffler_error_return_settings_upstream (upstream_service_id, is_active),
    CHECK (scope IN ('platform', 'upstream')),
    CHECK (response_mode IN ('replace', 'append', 'redact')),
    CHECK (account_protection_action IN ('record_only', 'pause_scheduling', 'disable_account')),
    CHECK (pause_duration IS NULL OR pause_duration IN ('ten_minutes', 'one_hour', 'twenty_four_hours', 'manual_restore'))
);

CREATE TABLE IF NOT EXISTS niffler_account_risk_events (
    id VARCHAR(36) NOT NULL,
    upstream_service_id VARCHAR(36),
    upstream_account_id VARCHAR(36) NOT NULL,
    request_id VARCHAR(100),
    user_id VARCHAR(36),
    api_key_id VARCHAR(36),
    model_name VARCHAR(200),
    rule_id VARCHAR(36),
    matched_text TEXT,
    upstream_status_code INT,
    action VARCHAR(32) NOT NULL,
    created_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_account_risk_events_account_time (upstream_account_id, created_at_unix_ms),
    INDEX idx_niffler_account_risk_events_request (request_id)
);

CREATE TABLE IF NOT EXISTS niffler_api_key_pauses (
    id VARCHAR(36) NOT NULL,
    api_key_id VARCHAR(36) NOT NULL,
    reason_code VARCHAR(64) NOT NULL,
    user_message TEXT NOT NULL,
    paused_until_unix_ms BIGINT,
    manual_restore_required BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_unix_ms BIGINT NOT NULL,
    restored_at_unix_ms BIGINT,
    restored_by VARCHAR(36),
    PRIMARY KEY (id),
    INDEX idx_niffler_api_key_pauses_key_active (api_key_id, restored_at_unix_ms, paused_until_unix_ms)
);
