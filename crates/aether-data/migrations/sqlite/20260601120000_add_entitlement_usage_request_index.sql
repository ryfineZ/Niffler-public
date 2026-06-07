CREATE INDEX IF NOT EXISTS idx_entitlement_usage_request
    ON entitlement_usage_ledgers (request_id);
