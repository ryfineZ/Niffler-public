CREATE TABLE IF NOT EXISTS public.niffler_upstream_service_capabilities (
    id character varying(36) PRIMARY KEY,
    upstream_service_id character varying(36) NOT NULL
        REFERENCES public.niffler_upstream_services (id),
    protocol_kind character varying(32) NOT NULL,
    capability_kind character varying(64) NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    config jsonb,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_upstream_service_capabilities_protocol
        CHECK (protocol_kind IN ('openai', 'anthropic', 'gemini', 'codex', 'custom')),
    CONSTRAINT ck_niffler_upstream_service_capabilities_kind
        CHECK (capability_kind IN ('text', 'streaming', 'images_endpoint', 'openai_responses_image_tool', 'model_list', 'model_test')),
    CONSTRAINT ck_niffler_upstream_service_capabilities_openai_tool
        CHECK (capability_kind <> 'openai_responses_image_tool' OR protocol_kind IN ('openai', 'codex')),
    CONSTRAINT uq_niffler_upstream_service_capabilities
        UNIQUE (upstream_service_id, protocol_kind, capability_kind)
);

CREATE INDEX IF NOT EXISTS idx_niffler_upstream_service_capabilities_lookup
    ON public.niffler_upstream_service_capabilities (protocol_kind, capability_kind, is_enabled);

CREATE TABLE IF NOT EXISTS public.niffler_settlement_snapshots (
    id character varying(36) PRIMARY KEY,
    request_id character varying(100) NOT NULL,
    user_id character varying(36),
    api_key_id character varying(36),
    product_plan_id character varying(36),
    upstream_service_id character varying(36),
    upstream_account_id character varying(36),
    requested_model_name character varying(200) NOT NULL,
    upstream_execution_model_name character varying(200),
    image_tool_model_name character varying(200),
    pricing_snapshot jsonb NOT NULL,
    wallet_charge_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    entitlement_charge_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    upstream_cost_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    gross_margin_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    created_at_unix_ms bigint NOT NULL,
    finalized_at_unix_ms bigint,
    CONSTRAINT uq_niffler_settlement_snapshots_request
        UNIQUE (request_id),
    CONSTRAINT ck_niffler_settlement_snapshots_non_negative
        CHECK (
            wallet_charge_usd >= 0
            AND entitlement_charge_usd >= 0
            AND upstream_cost_usd >= 0
        )
);

CREATE INDEX IF NOT EXISTS idx_niffler_settlement_snapshots_user_time
    ON public.niffler_settlement_snapshots (user_id, created_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS public.niffler_billing_reservations (
    id character varying(36) PRIMARY KEY,
    request_id character varying(100) NOT NULL,
    user_id character varying(36),
    api_key_id character varying(36),
    product_plan_id character varying(36),
    status character varying(32) NOT NULL,
    reserved_total_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    wallet_reserved_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    entitlement_reserved_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    reserved_at_unix_ms bigint NOT NULL,
    expires_at_unix_ms bigint NOT NULL,
    finalized_at_unix_ms bigint,
    settlement_snapshot_id character varying(36)
        REFERENCES public.niffler_settlement_snapshots (id),
    release_reason text,
    idempotency_key character varying(120) NOT NULL,
    CONSTRAINT uq_niffler_billing_reservations_request
        UNIQUE (request_id),
    CONSTRAINT uq_niffler_billing_reservations_idempotency
        UNIQUE (idempotency_key),
    CONSTRAINT ck_niffler_billing_reservations_status
        CHECK (status IN ('active', 'settled', 'released', 'expired', 'manual_review')),
    CONSTRAINT ck_niffler_billing_reservations_non_negative
        CHECK (
            reserved_total_usd >= 0
            AND wallet_reserved_usd >= 0
            AND entitlement_reserved_usd >= 0
        ),
    CONSTRAINT ck_niffler_billing_reservations_time_order
        CHECK (expires_at_unix_ms > reserved_at_unix_ms),
    CONSTRAINT ck_niffler_billing_reservations_settled_snapshot
        CHECK (status <> 'settled' OR settlement_snapshot_id IS NOT NULL),
    CONSTRAINT ck_niffler_billing_reservations_release_reason
        CHECK (
            status NOT IN ('released', 'expired', 'manual_review')
            OR (release_reason IS NOT NULL AND btrim(release_reason) <> '')
        )
);

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservations_status_expires
    ON public.niffler_billing_reservations (status, expires_at_unix_ms);

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservations_user_time
    ON public.niffler_billing_reservations (user_id, reserved_at_unix_ms DESC);

CREATE TABLE IF NOT EXISTS public.niffler_billing_reservation_events (
    id character varying(36) PRIMARY KEY,
    reservation_id character varying(36) NOT NULL
        REFERENCES public.niffler_billing_reservations (id),
    event_kind character varying(32) NOT NULL,
    amount_usd numeric(20, 8) DEFAULT 0 NOT NULL,
    reason text,
    idempotency_key character varying(120) NOT NULL,
    actor_id character varying(36),
    created_at_unix_ms bigint NOT NULL,
    CONSTRAINT uq_niffler_billing_reservation_events_idempotency
        UNIQUE (reservation_id, idempotency_key),
    CONSTRAINT ck_niffler_billing_reservation_events_kind
        CHECK (event_kind IN ('reserved', 'settled', 'released', 'expired', 'manual_review')),
    CONSTRAINT ck_niffler_billing_reservation_events_amount
        CHECK (amount_usd >= 0)
);

CREATE INDEX IF NOT EXISTS idx_niffler_billing_reservation_events_reservation
    ON public.niffler_billing_reservation_events (reservation_id, created_at_unix_ms);

CREATE TABLE IF NOT EXISTS public.niffler_referral_reward_rules (
    id character varying(36) PRIMARY KEY,
    display_name character varying(200) NOT NULL,
    status character varying(32) DEFAULT 'active' NOT NULL,
    reward_kind character varying(32) NOT NULL,
    reward_value numeric(20, 8) NOT NULL,
    applies_to_order_kind character varying(64),
    max_reward_usd numeric(20, 8),
    effective_from_unix_ms bigint NOT NULL,
    effective_until_unix_ms bigint,
    config jsonb,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_referral_reward_rules_status
        CHECK (status IN ('active', 'disabled')),
    CONSTRAINT ck_niffler_referral_reward_rules_kind
        CHECK (reward_kind IN ('fixed_amount', 'percentage')),
    CONSTRAINT ck_niffler_referral_reward_rules_non_negative
        CHECK (
            reward_value >= 0
            AND (reward_kind <> 'percentage' OR reward_value <= 1)
            AND (max_reward_usd IS NULL OR max_reward_usd >= 0)
        ),
    CONSTRAINT ck_niffler_referral_reward_rules_time_order
        CHECK (
            effective_until_unix_ms IS NULL
            OR effective_until_unix_ms > effective_from_unix_ms
        )
);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_rules_status_time
    ON public.niffler_referral_reward_rules (status, effective_from_unix_ms, effective_until_unix_ms);

CREATE TABLE IF NOT EXISTS public.niffler_referral_reward_ledger (
    id character varying(36) PRIMARY KEY,
    order_id character varying(100) NOT NULL,
    idempotency_key character varying(120) NOT NULL,
    inviter_user_id character varying(36) NOT NULL,
    invitee_user_id character varying(36) NOT NULL,
    rule_id character varying(36)
        REFERENCES public.niffler_referral_reward_rules (id),
    reward_amount_usd numeric(20, 8) NOT NULL,
    rule_snapshot jsonb NOT NULL,
    status character varying(32) NOT NULL,
    failure_reason text,
    retry_count integer DEFAULT 0 NOT NULL,
    paid_at_unix_ms bigint,
    cancelled_at_unix_ms bigint,
    created_at_unix_ms bigint NOT NULL,
    updated_at_unix_ms bigint NOT NULL,
    CONSTRAINT uq_niffler_referral_reward_ledger_order
        UNIQUE (order_id),
    CONSTRAINT uq_niffler_referral_reward_ledger_idempotency
        UNIQUE (idempotency_key),
    CONSTRAINT ck_niffler_referral_reward_ledger_status
        CHECK (status IN ('pending', 'paid', 'failed', 'cancelled')),
    CONSTRAINT ck_niffler_referral_reward_ledger_amount
        CHECK (reward_amount_usd >= 0),
    CONSTRAINT ck_niffler_referral_reward_ledger_retry
        CHECK (retry_count >= 0),
    CONSTRAINT ck_niffler_referral_reward_ledger_failed_reason
        CHECK (
            status <> 'failed'
            OR (failure_reason IS NOT NULL AND btrim(failure_reason) <> '')
        ),
    CONSTRAINT ck_niffler_referral_reward_ledger_paid_time
        CHECK (status <> 'paid' OR paid_at_unix_ms IS NOT NULL),
    CONSTRAINT ck_niffler_referral_reward_ledger_cancelled_time
        CHECK (status <> 'cancelled' OR cancelled_at_unix_ms IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_ledger_inviter
    ON public.niffler_referral_reward_ledger (inviter_user_id, created_at_unix_ms DESC);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_ledger_status
    ON public.niffler_referral_reward_ledger (status, updated_at_unix_ms);

CREATE TABLE IF NOT EXISTS public.niffler_referral_reward_events (
    id character varying(36) PRIMARY KEY,
    reward_ledger_id character varying(36) NOT NULL
        REFERENCES public.niffler_referral_reward_ledger (id),
    event_kind character varying(32) NOT NULL,
    reason text,
    actor_id character varying(36),
    event_snapshot jsonb,
    created_at_unix_ms bigint NOT NULL,
    CONSTRAINT ck_niffler_referral_reward_events_kind
        CHECK (event_kind IN ('created', 'paid', 'failed', 'retry_scheduled', 'manual_retry', 'manual_paid', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_niffler_referral_reward_events_ledger
    ON public.niffler_referral_reward_events (reward_ledger_id, created_at_unix_ms);
