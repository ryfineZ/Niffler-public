CREATE TABLE IF NOT EXISTS entitlement_usage_windows (
    id varchar(64) NOT NULL PRIMARY KEY,
    user_entitlement_id varchar(64) NOT NULL,
    user_id varchar(64) NOT NULL,
    window_scope varchar(32) NOT NULL,
    window_key varchar(64) NOT NULL,
    window_started_at bigint NOT NULL,
    window_ends_at bigint NOT NULL,
    used_usd decimal(20,8) NOT NULL DEFAULT 0,
    created_at bigint NOT NULL,
    updated_at bigint NOT NULL,
    UNIQUE KEY uq_entitlement_usage_window (user_entitlement_id, window_scope),
    KEY idx_entitlement_usage_windows_user_scope (user_id, window_scope, window_ends_at),
    CONSTRAINT entitlement_usage_windows_entitlement_fkey FOREIGN KEY (user_entitlement_id) REFERENCES user_plan_entitlements(id) ON DELETE CASCADE,
    CONSTRAINT entitlement_usage_windows_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
