CREATE TABLE IF NOT EXISTS niffler_upstream_service_capabilities (
    id VARCHAR(36) NOT NULL,
    upstream_service_id VARCHAR(36) NOT NULL,
    protocol_kind VARCHAR(32) NOT NULL,
    capability_kind VARCHAR(64) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    config JSON,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_upstream_service_capabilities (upstream_service_id, protocol_kind, capability_kind),
    INDEX idx_niffler_upstream_service_capabilities_lookup (protocol_kind, capability_kind, is_enabled),
    CONSTRAINT fk_niffler_upstream_service_capabilities_service
        FOREIGN KEY (upstream_service_id) REFERENCES niffler_upstream_services (id),
    CHECK (protocol_kind IN ('openai', 'anthropic', 'gemini', 'codex', 'custom')),
    CHECK (capability_kind IN ('text', 'streaming', 'images_endpoint', 'openai_responses_image_tool', 'model_list', 'model_test')),
    CHECK (capability_kind <> 'openai_responses_image_tool' OR protocol_kind IN ('openai', 'codex'))
);

CREATE TABLE IF NOT EXISTS niffler_settlement_snapshots (
    id VARCHAR(36) NOT NULL,
    request_id VARCHAR(100) NOT NULL,
    user_id VARCHAR(36),
    api_key_id VARCHAR(36),
    product_plan_id VARCHAR(36),
    upstream_service_id VARCHAR(36),
    upstream_account_id VARCHAR(36),
    requested_model_name VARCHAR(200) NOT NULL,
    upstream_execution_model_name VARCHAR(200),
    image_tool_model_name VARCHAR(200),
    pricing_snapshot JSON NOT NULL,
    wallet_charge_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    entitlement_charge_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    upstream_cost_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    gross_margin_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    created_at_unix_ms BIGINT NOT NULL,
    finalized_at_unix_ms BIGINT,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_settlement_snapshots_request (request_id),
    INDEX idx_niffler_settlement_snapshots_user_time (user_id, created_at_unix_ms),
    CHECK (
        wallet_charge_usd >= 0
        AND entitlement_charge_usd >= 0
        AND upstream_cost_usd >= 0
    )
);

CREATE TABLE IF NOT EXISTS niffler_billing_reservations (
    id VARCHAR(36) NOT NULL,
    request_id VARCHAR(100) NOT NULL,
    user_id VARCHAR(36),
    api_key_id VARCHAR(36),
    product_plan_id VARCHAR(36),
    status VARCHAR(32) NOT NULL,
    reserved_total_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    wallet_reserved_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    entitlement_reserved_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    reserved_at_unix_ms BIGINT NOT NULL,
    expires_at_unix_ms BIGINT NOT NULL,
    finalized_at_unix_ms BIGINT,
    settlement_snapshot_id VARCHAR(36),
    release_reason TEXT,
    idempotency_key VARCHAR(120) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_billing_reservations_request (request_id),
    UNIQUE KEY uq_niffler_billing_reservations_idempotency (idempotency_key),
    INDEX idx_niffler_billing_reservations_status_expires (status, expires_at_unix_ms),
    INDEX idx_niffler_billing_reservations_user_time (user_id, reserved_at_unix_ms),
    CONSTRAINT fk_niffler_billing_reservations_snapshot
        FOREIGN KEY (settlement_snapshot_id) REFERENCES niffler_settlement_snapshots (id),
    CHECK (status IN ('active', 'settled', 'released', 'expired', 'manual_review')),
    CHECK (
        reserved_total_usd >= 0
        AND wallet_reserved_usd >= 0
        AND entitlement_reserved_usd >= 0
    ),
    CHECK (expires_at_unix_ms > reserved_at_unix_ms),
    CHECK (status <> 'settled' OR settlement_snapshot_id IS NOT NULL),
    CHECK (
        status NOT IN ('released', 'expired', 'manual_review')
        OR (release_reason IS NOT NULL AND TRIM(release_reason) <> '')
    )
);

CREATE TABLE IF NOT EXISTS niffler_billing_reservation_events (
    id VARCHAR(36) NOT NULL,
    reservation_id VARCHAR(36) NOT NULL,
    event_kind VARCHAR(32) NOT NULL,
    amount_usd DECIMAL(20, 8) NOT NULL DEFAULT 0,
    reason TEXT,
    idempotency_key VARCHAR(120) NOT NULL,
    actor_id VARCHAR(36),
    created_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_billing_reservation_events_idempotency (reservation_id, idempotency_key),
    INDEX idx_niffler_billing_reservation_events_reservation (reservation_id, created_at_unix_ms),
    CONSTRAINT fk_niffler_billing_reservation_events_reservation
        FOREIGN KEY (reservation_id) REFERENCES niffler_billing_reservations (id),
    CHECK (event_kind IN ('reserved', 'settled', 'released', 'expired', 'manual_review')),
    CHECK (amount_usd >= 0)
);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_rules (
    id VARCHAR(36) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    reward_kind VARCHAR(32) NOT NULL,
    reward_value DECIMAL(20, 8) NOT NULL,
    applies_to_order_kind VARCHAR(64),
    max_reward_usd DECIMAL(20, 8),
    effective_from_unix_ms BIGINT NOT NULL,
    effective_until_unix_ms BIGINT,
    config JSON,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_referral_reward_rules_status_time (status, effective_from_unix_ms, effective_until_unix_ms),
    CHECK (status IN ('active', 'disabled')),
    CHECK (reward_kind IN ('fixed_amount', 'percentage')),
    CHECK (
        reward_value >= 0
        AND (reward_kind <> 'percentage' OR reward_value <= 1)
        AND (max_reward_usd IS NULL OR max_reward_usd >= 0)
    ),
    CHECK (
        effective_until_unix_ms IS NULL
        OR effective_until_unix_ms > effective_from_unix_ms
    )
);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_ledger (
    id VARCHAR(36) NOT NULL,
    order_id VARCHAR(100) NOT NULL,
    idempotency_key VARCHAR(120) NOT NULL,
    inviter_user_id VARCHAR(36) NOT NULL,
    invitee_user_id VARCHAR(36) NOT NULL,
    rule_id VARCHAR(36),
    reward_amount_usd DECIMAL(20, 8) NOT NULL,
    rule_snapshot JSON NOT NULL,
    status VARCHAR(32) NOT NULL,
    failure_reason TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    paid_at_unix_ms BIGINT,
    cancelled_at_unix_ms BIGINT,
    created_at_unix_ms BIGINT NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_niffler_referral_reward_ledger_order (order_id),
    UNIQUE KEY uq_niffler_referral_reward_ledger_idempotency (idempotency_key),
    INDEX idx_niffler_referral_reward_ledger_inviter (inviter_user_id, created_at_unix_ms),
    INDEX idx_niffler_referral_reward_ledger_status (status, updated_at_unix_ms),
    CONSTRAINT fk_niffler_referral_reward_ledger_rule
        FOREIGN KEY (rule_id) REFERENCES niffler_referral_reward_rules (id),
    CHECK (status IN ('pending', 'paid', 'failed', 'cancelled')),
    CHECK (reward_amount_usd >= 0),
    CHECK (retry_count >= 0),
    CHECK (
        status <> 'failed'
        OR (failure_reason IS NOT NULL AND TRIM(failure_reason) <> '')
    ),
    CHECK (status <> 'paid' OR paid_at_unix_ms IS NOT NULL),
    CHECK (status <> 'cancelled' OR cancelled_at_unix_ms IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_events (
    id VARCHAR(36) NOT NULL,
    reward_ledger_id VARCHAR(36) NOT NULL,
    event_kind VARCHAR(32) NOT NULL,
    reason TEXT,
    actor_id VARCHAR(36),
    event_snapshot JSON,
    created_at_unix_ms BIGINT NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_niffler_referral_reward_events_ledger (reward_ledger_id, created_at_unix_ms),
    CONSTRAINT fk_niffler_referral_reward_events_ledger
        FOREIGN KEY (reward_ledger_id) REFERENCES niffler_referral_reward_ledger (id),
    CHECK (event_kind IN ('created', 'paid', 'failed', 'retry_scheduled', 'manual_retry', 'manual_paid', 'cancelled'))
);
