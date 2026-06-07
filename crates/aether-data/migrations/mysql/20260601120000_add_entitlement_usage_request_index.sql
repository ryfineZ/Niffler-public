SET @aether_entitlement_usage_request_index_sql := IF(
    (
        SELECT COUNT(*)
        FROM information_schema.statistics
        WHERE table_schema = DATABASE()
          AND table_name = 'entitlement_usage_ledgers'
          AND index_name = 'idx_entitlement_usage_request'
    ) = 0,
    'CREATE INDEX idx_entitlement_usage_request ON entitlement_usage_ledgers (request_id)',
    'DO 0'
);

PREPARE aether_entitlement_usage_request_index_stmt FROM @aether_entitlement_usage_request_index_sql;
EXECUTE aether_entitlement_usage_request_index_stmt;
DEALLOCATE PREPARE aether_entitlement_usage_request_index_stmt;
