CREATE TABLE IF NOT EXISTS niffler_upstream_service_capabilities (
    id TEXT PRIMARY KEY,
    upstream_service_id TEXT NOT NULL REFERENCES niffler_upstream_services (id),
    protocol_kind TEXT NOT NULL,
    capability_kind TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    config TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE (upstream_service_id, protocol_kind, capability_kind),
    CHECK (protocol_kind IN ('openai', 'anthropic', 'gemini', 'codex', 'custom')),
    CHECK (capability_kind IN ('text', 'streaming', 'images_endpoint', 'openai_responses_image_tool', 'model_list', 'model_test')),
    CHECK (capability_kind <> 'openai_responses_image_tool' OR protocol_kind IN ('openai', 'codex'))
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_service_capabilities_lookup
    ON niffler_upstream_service_capabilities (protocol_kind, capability_kind, is_enabled);

CREATE TABLE IF NOT EXISTS niffler_settlement_snapshots (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    user_id TEXT,
    api_key_id TEXT,
    product_plan_id TEXT,
    upstream_service_id TEXT,
    upstream_account_id TEXT,
    requested_model_name TEXT NOT NULL,
    upstream_execution_model_name TEXT,
    image_tool_model_name TEXT,
    pricing_snapshot TEXT NOT NULL,
    wallet_charge_usd REAL NOT NULL DEFAULT 0,
    entitlement_charge_usd REAL NOT NULL DEFAULT 0,
    upstream_cost_usd REAL NOT NULL DEFAULT 0,
    gross_margin_usd REAL NOT NULL DEFAULT 0,
    created_at_unix_ms INTEGER NOT NULL,
    finalized_at_unix_ms INTEGER,
    UNIQUE (request_id),
    CHECK (
        wallet_charge_usd >= 0
        AND entitlement_charge_usd >= 0
        AND upstream_cost_usd >= 0
    )
);

CREATE INDEX IF NOT EXISTS idx_niffler_settlement_snapshots_user_time
    ON niffler_settlement_snapshots (user_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS niffler_billing_reservations (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    user_id TEXT,
    api_key_id TEXT,
    product_plan_id TEXT,
    status TEXT NOT NULL,
    reserved_total_usd REAL NOT NULL DEFAULT 0,
    wallet_reserved_usd REAL NOT NULL DEFAULT 0,
    entitlement_reserved_usd REAL NOT NULL DEFAULT 0,
    reserved_at_unix_ms INTEGER NOT NULL,
    expires_at_unix_ms INTEGER NOT NULL,
    finalized_at_unix_ms INTEGER,
    settlement_snapshot_id TEXT REFERENCES niffler_settlement_snapshots (id),
    release_reason TEXT,
    idempotency_key TEXT NOT NULL,
    UNIQUE (request_id),
    UNIQUE (idempotency_key),
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

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservations_status_expires
    ON niffler_billing_reservations (status, expires_at_unix_ms);

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservations_user_time
    ON niffler_billing_reservations (user_id, reserved_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS niffler_billing_reservation_events (
    id TEXT PRIMARY KEY,
    reservation_id TEXT NOT NULL REFERENCES niffler_billing_reservations (id),
    event_kind TEXT NOT NULL,
    amount_usd REAL NOT NULL DEFAULT 0,
    reason TEXT,
    idempotency_key TEXT NOT NULL,
    actor_id TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    UNIQUE (reservation_id, idempotency_key),
    CHECK (event_kind IN ('reserved', 'settled', 'released', 'expired', 'manual_review')),
    CHECK (amount_usd >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservation_events_reservation
    ON niffler_billing_reservation_events (reservation_id, created_at_unix_ms);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_rules (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    reward_kind TEXT NOT NULL,
    reward_value REAL NOT NULL,
    applies_to_order_kind TEXT,
    max_reward_usd REAL,
    effective_from_unix_ms INTEGER NOT NULL,
    effective_until_unix_ms INTEGER,
    config TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_rules_status_time
    ON niffler_referral_reward_rules (status, effective_from_unix_ms, effective_until_unix_ms);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_ledger (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    inviter_user_id TEXT NOT NULL,
    invitee_user_id TEXT NOT NULL,
    rule_id TEXT REFERENCES niffler_referral_reward_rules (id),
    reward_amount_usd REAL NOT NULL,
    rule_snapshot TEXT NOT NULL,
    status TEXT NOT NULL,
    failure_reason TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    paid_at_unix_ms INTEGER,
    cancelled_at_unix_ms INTEGER,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    UNIQUE (order_id),
    UNIQUE (idempotency_key),
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

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_ledger_inviter
    ON niffler_referral_reward_ledger (inviter_user_id, created_at_unix_ms DESC);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_ledger_status
    ON niffler_referral_reward_ledger (status, updated_at_unix_ms);

CREATE TABLE IF NOT EXISTS niffler_referral_reward_events (
    id TEXT PRIMARY KEY,
    reward_ledger_id TEXT NOT NULL REFERENCES niffler_referral_reward_ledger (id),
    event_kind TEXT NOT NULL,
    reason TEXT,
    actor_id TEXT,
    event_snapshot TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    CHECK (event_kind IN ('created', 'paid', 'failed', 'retry_scheduled', 'manual_retry', 'manual_paid', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_events_ledger
    ON niffler_referral_reward_events (reward_ledger_id, created_at_unix_ms);
