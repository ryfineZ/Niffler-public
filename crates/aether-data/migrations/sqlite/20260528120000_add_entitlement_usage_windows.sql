CREATE TABLE IF NOT EXISTS entitlement_usage_windows (
    id TEXT PRIMARY KEY,
    user_entitlement_id TEXT NOT NULL REFERENCES user_plan_entitlements(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    window_scope TEXT NOT NULL,
    window_key TEXT NOT NULL,
    window_started_at INTEGER NOT NULL,
    window_ends_at INTEGER NOT NULL,
    used_usd REAL NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (user_entitlement_id, window_scope)
);

CREATE INDEX IF NOT EXISTS idx_entitlement_usage_windows_user_scope
  ON entitlement_usage_windows (user_id, window_scope, window_ends_at);
