use async_trait::async_trait;
use sqlx::{PgPool, Row};

use super::{
    finite_wallet_available_usd, plan_finite_wallet_debit,
    settlement_billing_status_for_usage_status, settlement_wallet_charge_multiplier,
    SettlementWriteRepository, StoredUsageSettlement, UsageSettlementInput, SETTLEMENT_EPSILON_USD,
};
use crate::driver::postgres::PostgresTransactionRunner;
use crate::error::SqlxResultExt;
use crate::repository::billing::quota::{
    entitlement_allows_global_model, entitlements_snapshot_has_usage_quota_for_global_model,
    usage_quota_grants_from_entitlement, StoredUsageQuotaWindow, UsageQuotaGrant,
    QUOTA_SCOPE_FIVE_HOUR,
};
use crate::DataLayerError;

const FIND_USAGE_FOR_SETTLEMENT_SQL: &str = r#"
SELECT
  usage_record.request_id,
  COALESCE(usage_settlement_snapshots.wallet_id, usage_record.wallet_id) AS wallet_id,
  COALESCE(usage_settlement_snapshots.billing_status, usage_record.billing_status) AS billing_status,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_balance_before AS DOUBLE PRECISION),
    CAST(usage_record.wallet_balance_before AS DOUBLE PRECISION)
  ) AS wallet_balance_before,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_balance_after AS DOUBLE PRECISION),
    CAST(usage_record.wallet_balance_after AS DOUBLE PRECISION)
  ) AS wallet_balance_after,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_recharge_balance_before AS DOUBLE PRECISION),
    CAST(usage_record.wallet_recharge_balance_before AS DOUBLE PRECISION)
  ) AS wallet_recharge_balance_before,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_recharge_balance_after AS DOUBLE PRECISION),
    CAST(usage_record.wallet_recharge_balance_after AS DOUBLE PRECISION)
  ) AS wallet_recharge_balance_after,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_gift_balance_before AS DOUBLE PRECISION),
    CAST(usage_record.wallet_gift_balance_before AS DOUBLE PRECISION)
  ) AS wallet_gift_balance_before,
  COALESCE(
    CAST(usage_settlement_snapshots.wallet_gift_balance_after AS DOUBLE PRECISION),
    CAST(usage_record.wallet_gift_balance_after AS DOUBLE PRECISION)
  ) AS wallet_gift_balance_after,
  CAST(usage_settlement_snapshots.provider_monthly_used_usd AS DOUBLE PRECISION) AS provider_monthly_used_usd,
  usage_record.provider_id,
  CAST(
    EXTRACT(
      EPOCH FROM COALESCE(usage_settlement_snapshots.finalized_at, usage_record.finalized_at)
    ) AS BIGINT
  ) AS finalized_at_unix_secs
FROM "usage" AS usage_record
LEFT JOIN usage_settlement_snapshots
  ON usage_settlement_snapshots.request_id = usage_record.request_id
WHERE usage_record.request_id = $1
FOR UPDATE OF usage_record
"#;

const FINALIZE_USAGE_BILLING_SQL: &str = r#"
UPDATE "usage"
SET
  billing_status = $2,
  finalized_at = COALESCE(finalized_at, to_timestamp($3))
WHERE request_id = $1
"#;

const UPSERT_USAGE_SETTLEMENT_SNAPSHOT_SQL: &str = r#"
INSERT INTO usage_settlement_snapshots (
  request_id,
  billing_status,
  wallet_id,
  wallet_balance_before,
  wallet_balance_after,
  wallet_recharge_balance_before,
  wallet_recharge_balance_after,
  wallet_gift_balance_before,
  wallet_gift_balance_after,
  provider_monthly_used_usd,
  finalized_at
) VALUES (
  $1,
  $2,
  $3,
  $4,
  $5,
  $6,
  $7,
  $8,
  $9,
  $10,
  CASE
    WHEN $11 IS NULL THEN NULL
    ELSE TO_TIMESTAMP($11::double precision)
  END
)
ON CONFLICT (request_id)
DO UPDATE SET
  billing_status = EXCLUDED.billing_status,
  wallet_id = COALESCE(EXCLUDED.wallet_id, usage_settlement_snapshots.wallet_id),
  wallet_balance_before = COALESCE(
    EXCLUDED.wallet_balance_before,
    usage_settlement_snapshots.wallet_balance_before
  ),
  wallet_balance_after = COALESCE(
    EXCLUDED.wallet_balance_after,
    usage_settlement_snapshots.wallet_balance_after
  ),
  wallet_recharge_balance_before = COALESCE(
    EXCLUDED.wallet_recharge_balance_before,
    usage_settlement_snapshots.wallet_recharge_balance_before
  ),
  wallet_recharge_balance_after = COALESCE(
    EXCLUDED.wallet_recharge_balance_after,
    usage_settlement_snapshots.wallet_recharge_balance_after
  ),
  wallet_gift_balance_before = COALESCE(
    EXCLUDED.wallet_gift_balance_before,
    usage_settlement_snapshots.wallet_gift_balance_before
  ),
  wallet_gift_balance_after = COALESCE(
    EXCLUDED.wallet_gift_balance_after,
    usage_settlement_snapshots.wallet_gift_balance_after
  ),
  provider_monthly_used_usd = COALESCE(
    EXCLUDED.provider_monthly_used_usd,
    usage_settlement_snapshots.provider_monthly_used_usd
  ),
  finalized_at = COALESCE(EXCLUDED.finalized_at, usage_settlement_snapshots.finalized_at),
  updated_at = NOW()
"#;

const ENQUEUE_PROVIDER_MONTHLY_USAGE_DELTA_SQL: &str = r#"
INSERT INTO usage_counter_deltas (
  id,
  request_id,
  kind,
  target_id,
  total_cost_usd_delta
) VALUES (
  $1,
  $2,
  'provider_monthly',
  $3,
  $4
)
"#;

#[derive(Debug, Clone)]
pub struct SqlxSettlementRepository {
    tx_runner: PostgresTransactionRunner,
}

impl SqlxSettlementRepository {
    pub fn new(pool: PgPool) -> Self {
        let tx_runner = PostgresTransactionRunner::new(pool);
        Self { tx_runner }
    }
}

fn settlement_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<StoredUsageSettlement, DataLayerError> {
    Ok(StoredUsageSettlement {
        request_id: row.try_get("request_id").map_postgres_err()?,
        wallet_id: row.try_get("wallet_id").map_postgres_err()?,
        billing_status: row.try_get("billing_status").map_postgres_err()?,
        wallet_balance_before: row.try_get("wallet_balance_before").map_postgres_err()?,
        wallet_balance_after: row.try_get("wallet_balance_after").map_postgres_err()?,
        wallet_recharge_balance_before: row
            .try_get("wallet_recharge_balance_before")
            .map_postgres_err()?,
        wallet_recharge_balance_after: row
            .try_get("wallet_recharge_balance_after")
            .map_postgres_err()?,
        wallet_gift_balance_before: row
            .try_get("wallet_gift_balance_before")
            .map_postgres_err()?,
        wallet_gift_balance_after: row
            .try_get("wallet_gift_balance_after")
            .map_postgres_err()?,
        provider_monthly_used_usd: row
            .try_get("provider_monthly_used_usd")
            .map_postgres_err()?,
        finalized_at_unix_secs: row
            .try_get::<Option<i64>, _>("finalized_at_unix_secs")
            .map_postgres_err()?
            .map(|value| value as u64),
    })
}

async fn sync_usage_settlement_snapshot<'e, E>(
    executor: E,
    settlement: &StoredUsageSettlement,
) -> Result<(), DataLayerError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(UPSERT_USAGE_SETTLEMENT_SNAPSHOT_SQL)
        .bind(&settlement.request_id)
        .bind(&settlement.billing_status)
        .bind(settlement.wallet_id.as_deref())
        .bind(settlement.wallet_balance_before)
        .bind(settlement.wallet_balance_after)
        .bind(settlement.wallet_recharge_balance_before)
        .bind(settlement.wallet_recharge_balance_after)
        .bind(settlement.wallet_gift_balance_before)
        .bind(settlement.wallet_gift_balance_after)
        .bind(settlement.provider_monthly_used_usd)
        .bind(settlement.finalized_at_unix_secs.map(|value| value as f64))
        .execute(executor)
        .await
        .map_postgres_err()?;
    Ok(())
}

async fn enqueue_provider_monthly_usage_delta<'e, E>(
    executor: E,
    request_id: &str,
    provider_id: &str,
    total_cost_usd_delta: f64,
) -> Result<(), DataLayerError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let request_id = request_id.trim();
    let provider_id = provider_id.trim();
    if request_id.is_empty() || provider_id.is_empty() || total_cost_usd_delta == 0.0 {
        return Ok(());
    }
    if !total_cost_usd_delta.is_finite() {
        return Err(DataLayerError::UnexpectedValue(format!(
            "provider monthly usage delta is not finite for {provider_id}"
        )));
    }

    sqlx::query(ENQUEUE_PROVIDER_MONTHLY_USAGE_DELTA_SQL)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(request_id)
        .bind(provider_id)
        .bind(total_cost_usd_delta)
        .execute(executor)
        .await
        .map_postgres_err()?;
    Ok(())
}

#[derive(Debug, Default)]
struct DailyQuotaDebitResult {
    debited_usd: f64,
    insufficient: bool,
}

struct DailyQuotaDebitInput<'a> {
    user_id: &'a str,
    request_id: &'a str,
    total_cost_usd: f64,
    wallet_available_usd: Option<f64>,
    wallet_can_overdraft: bool,
    wallet_charge_multiplier: f64,
    request_global_model_id: Option<&'a str>,
}

async fn consume_daily_quota_postgres(
    tx: &mut crate::driver::postgres::PostgresTransaction,
    input: DailyQuotaDebitInput<'_>,
) -> Result<DailyQuotaDebitResult, DataLayerError> {
    if input.total_cost_usd <= 0.0 {
        return Ok(DailyQuotaDebitResult::default());
    }
    let now = chrono::Utc::now();
    let entitlement_rows = sqlx::query(
        r#"
SELECT id, starts_at, entitlements_snapshot
FROM user_plan_entitlements
WHERE user_id = $1
  AND status = 'active'
  AND starts_at <= NOW()
  AND expires_at > NOW()
ORDER BY expires_at ASC, created_at ASC, id ASC
FOR UPDATE
        "#,
    )
    .bind(input.user_id)
    .fetch_all(&mut **tx)
    .await
    .map_postgres_err()?;
    let mut grants = Vec::new();
    for row in entitlement_rows {
        let entitlement_id: String = row.try_get("id").map_postgres_err()?;
        let entitlement_started_at: chrono::DateTime<chrono::Utc> =
            row.try_get("starts_at").map_postgres_err()?;
        let entitlements: serde_json::Value =
            row.try_get("entitlements_snapshot").map_postgres_err()?;
        if input.request_global_model_id.is_some()
            && !entitlements_snapshot_has_usage_quota_for_global_model(
                &entitlements,
                input.request_global_model_id,
            )
        {
            continue;
        }
        let stored_five_hour =
            find_usage_quota_window_postgres(tx, &entitlement_id, QUOTA_SCOPE_FIVE_HOUR).await?;
        grants.extend(usage_quota_grants_from_entitlement(
            &entitlement_id,
            &entitlements,
            now,
            entitlement_started_at,
            stored_five_hour.as_ref(),
        )?);
    }
    grants.retain(|grant| {
        entitlement_allows_global_model(
            grant.allowed_global_model_ids.as_deref(),
            input.request_global_model_id,
        )
    });
    if grants.is_empty() {
        return Ok(DailyQuotaDebitResult::default());
    }

    let mut grants_by_entitlement: std::collections::BTreeMap<String, Vec<(UsageQuotaGrant, f64)>> =
        std::collections::BTreeMap::new();
    let mut total_remaining = 0.0;
    let mut allow_wallet_overage = true;
    for grant in grants {
        allow_wallet_overage &= grant.allow_wallet_overage;
        let used = upsert_usage_quota_window_postgres(tx, input.user_id, &grant).await?;
        let remaining = (grant.limit_usd - used).max(0.0);
        grants_by_entitlement
            .entry(grant.entitlement_id.clone())
            .or_default()
            .push((grant, remaining));
    }
    let mut entitlement_remaining = Vec::new();
    for grants in grants_by_entitlement.into_values() {
        let remaining = grants
            .iter()
            .map(|(_, remaining)| *remaining)
            .fold(f64::INFINITY, f64::min);
        if remaining.is_finite() {
            total_remaining += remaining.max(0.0);
            entitlement_remaining.push((grants, remaining.max(0.0)));
        }
    }

    if !allow_wallet_overage && total_remaining + 0.000_000_01 < input.total_cost_usd {
        return Ok(DailyQuotaDebitResult {
            debited_usd: 0.0,
            insufficient: true,
        });
    }
    if allow_wallet_overage
        && !input.wallet_can_overdraft
        && input.wallet_available_usd.is_some_and(|available| {
            available + SETTLEMENT_EPSILON_USD
                < (input.total_cost_usd - total_remaining).max(0.0) * input.wallet_charge_multiplier
        })
    {
        return Ok(DailyQuotaDebitResult {
            debited_usd: 0.0,
            insufficient: true,
        });
    }

    let mut remaining_cost = input.total_cost_usd;
    let mut debited = 0.0;
    for (grants, balance_before) in entitlement_remaining {
        if remaining_cost <= 0.000_000_01 || balance_before <= 0.0 {
            continue;
        }
        let amount = remaining_cost.min(balance_before);
        let balance_after = balance_before - amount;
        for (grant, _) in &grants {
            increment_usage_quota_window_postgres(tx, grant, amount).await?;
        }
        let primary_grant = &grants[0].0;
        sqlx::query(
            r#"
INSERT INTO entitlement_usage_ledgers (
  id, user_entitlement_id, user_id, request_id, amount_usd,
  balance_before, balance_after, usage_date, created_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
ON CONFLICT (user_entitlement_id, request_id) DO NOTHING
            "#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&primary_grant.entitlement_id)
        .bind(input.user_id)
        .bind(input.request_id)
        .bind(amount)
        .bind(balance_before)
        .bind(balance_after)
        .bind(&primary_grant.window_key)
        .execute(&mut **tx)
        .await
        .map_postgres_err()?;
        remaining_cost -= amount;
        debited += amount;
    }
    Ok(DailyQuotaDebitResult {
        debited_usd: debited,
        insufficient: false,
    })
}

async fn find_usage_quota_window_postgres(
    tx: &mut crate::driver::postgres::PostgresTransaction,
    entitlement_id: &str,
    scope: &str,
) -> Result<Option<StoredUsageQuotaWindow>, DataLayerError> {
    let row = sqlx::query(
        r#"
SELECT
  window_key,
  window_started_at,
  window_ends_at,
  CAST(used_usd AS DOUBLE PRECISION) AS used_usd
FROM entitlement_usage_windows
WHERE user_entitlement_id = $1
  AND window_scope = $2
LIMIT 1
        "#,
    )
    .bind(entitlement_id)
    .bind(scope)
    .fetch_optional(&mut **tx)
    .await
    .map_postgres_err()?;
    row.map(|row| {
        Ok(StoredUsageQuotaWindow {
            window_key: row.try_get("window_key").map_postgres_err()?,
            window_started_at: row.try_get("window_started_at").map_postgres_err()?,
            window_ends_at: row.try_get("window_ends_at").map_postgres_err()?,
        })
    })
    .transpose()
}

async fn upsert_usage_quota_window_postgres(
    tx: &mut crate::driver::postgres::PostgresTransaction,
    user_id: &str,
    grant: &crate::repository::billing::quota::UsageQuotaGrant,
) -> Result<f64, DataLayerError> {
    let row = sqlx::query(
        r#"
INSERT INTO entitlement_usage_windows (
  id, user_entitlement_id, user_id, window_scope, window_key,
  window_started_at, window_ends_at, used_usd, created_at, updated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, 0, NOW(), NOW())
ON CONFLICT (user_entitlement_id, window_scope)
DO UPDATE SET
  window_key = CASE
    WHEN entitlement_usage_windows.window_ends_at <= NOW()
      THEN EXCLUDED.window_key
    ELSE entitlement_usage_windows.window_key
  END,
  window_started_at = CASE
    WHEN entitlement_usage_windows.window_ends_at <= NOW()
      THEN EXCLUDED.window_started_at
    ELSE entitlement_usage_windows.window_started_at
  END,
  window_ends_at = CASE
    WHEN entitlement_usage_windows.window_ends_at <= NOW()
      THEN EXCLUDED.window_ends_at
    ELSE entitlement_usage_windows.window_ends_at
  END,
  used_usd = CASE
    WHEN entitlement_usage_windows.window_ends_at <= NOW()
      THEN 0
    ELSE entitlement_usage_windows.used_usd
  END,
  updated_at = NOW()
RETURNING CAST(used_usd AS DOUBLE PRECISION) AS used_usd
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&grant.entitlement_id)
    .bind(user_id)
    .bind(grant.scope)
    .bind(&grant.window_key)
    .bind(grant.window_started_at)
    .bind(grant.window_ends_at)
    .fetch_one(&mut **tx)
    .await
    .map_postgres_err()?;
    row.try_get("used_usd").map_postgres_err()
}

async fn increment_usage_quota_window_postgres(
    tx: &mut crate::driver::postgres::PostgresTransaction,
    grant: &crate::repository::billing::quota::UsageQuotaGrant,
    amount: f64,
) -> Result<(), DataLayerError> {
    sqlx::query(
        r#"
UPDATE entitlement_usage_windows
SET used_usd = used_usd + $3,
    updated_at = NOW()
WHERE user_entitlement_id = $1
  AND window_scope = $2
        "#,
    )
    .bind(&grant.entitlement_id)
    .bind(grant.scope)
    .bind(amount)
    .execute(&mut **tx)
    .await
    .map_postgres_err()?;
    Ok(())
}

#[async_trait]
impl SettlementWriteRepository for SqlxSettlementRepository {
    async fn settle_usage(
        &self,
        input: UsageSettlementInput,
    ) -> Result<Option<StoredUsageSettlement>, DataLayerError> {
        input.validate()?;
        self.tx_runner
            .run_read_write(|tx| {
                Box::pin(async move {
                    let row = sqlx::query(FIND_USAGE_FOR_SETTLEMENT_SQL)
                        .bind(&input.request_id)
                        .fetch_optional(&mut **tx)
                        .await
                        .map_postgres_err()?;

                    let Some(usage_row) = row else {
                        return Ok(None);
                    };

                    let current_billing_status: String =
                        usage_row.try_get("billing_status").map_postgres_err()?;
                    if matches!(
                        current_billing_status.as_str(),
                        "settled" | "void" | "insufficient_quota"
                    ) {
                        return settlement_from_row(&usage_row).map(Some);
                    }

                    let mut final_billing_status =
                        settlement_billing_status_for_usage_status(&input.status).to_string();
                    let finalized_at =
                        i64::try_from(input.finalized_at_unix_secs.unwrap_or_else(|| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        }))
                        .map_err(|_| {
                            DataLayerError::InvalidInput("finalized_at overflow".to_string())
                        })?;

                    let mut settlement = StoredUsageSettlement {
                        request_id: input.request_id.clone(),
                        wallet_id: None,
                        billing_status: final_billing_status.to_string(),
                        wallet_balance_before: None,
                        wallet_balance_after: None,
                        wallet_recharge_balance_before: None,
                        wallet_recharge_balance_after: None,
                        wallet_gift_balance_before: None,
                        wallet_gift_balance_after: None,
                        provider_monthly_used_usd: None,
                        finalized_at_unix_secs: Some(finalized_at as u64),
                    };

                    if final_billing_status == "settled" {
                        let api_key_id = input
                            .api_key_id
                            .as_deref()
                            .filter(|value| !value.is_empty());
                        let api_key_is_standalone = if input.api_key_is_standalone {
                            true
                        } else if let Some(api_key_id) = api_key_id {
                            sqlx::query_scalar::<_, bool>(
                                r#"
SELECT is_standalone
FROM api_keys
WHERE id = $1
LIMIT 1
                                "#,
                            )
                            .bind(api_key_id)
                            .fetch_optional(&mut **tx)
                            .await
                            .map_postgres_err()?
                            .unwrap_or(false)
                        } else {
                            false
                        };

                        let wallet_row = if let Some(api_key_id) = api_key_id {
                            sqlx::query(
                                r#"
SELECT
  id,
  CAST(balance AS DOUBLE PRECISION) AS balance,
  CAST(gift_balance AS DOUBLE PRECISION) AS gift_balance,
  limit_mode
FROM wallets
WHERE api_key_id = $1
FOR UPDATE
LIMIT 1
                                "#,
                            )
                            .bind(api_key_id)
                            .fetch_optional(&mut **tx)
                            .await
                            .map_postgres_err()?
                        } else {
                            None
                        };

                        let wallet_row = if wallet_row.is_some() {
                            wallet_row
                        } else if !api_key_is_standalone {
                            if let Some(user_id) =
                                input.user_id.as_deref().filter(|value| !value.is_empty())
                            {
                                sqlx::query(
                                    r#"
SELECT
  id,
  CAST(balance AS DOUBLE PRECISION) AS balance,
  CAST(gift_balance AS DOUBLE PRECISION) AS gift_balance,
  limit_mode
FROM wallets
WHERE user_id = $1
FOR UPDATE
LIMIT 1
                                    "#,
                                )
                                .bind(user_id)
                                .fetch_optional(&mut **tx)
                                .await
                                .map_postgres_err()?
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let wallet_can_overdraft = wallet_row.is_some();
                        let wallet_available_usd = match wallet_row.as_ref() {
                            Some(row) => {
                                let limit_mode: String =
                                    row.try_get("limit_mode").map_postgres_err()?;
                                if limit_mode.eq_ignore_ascii_case("unlimited") {
                                    None
                                } else {
                                    Some(finite_wallet_available_usd(
                                        row.try_get("balance").map_postgres_err()?,
                                        row.try_get("gift_balance").map_postgres_err()?,
                                    ))
                                }
                            }
                            None => Some(0.0),
                        };
                        if let Some(row) = wallet_row.as_ref() {
                            let wallet_id: String = row.try_get("id").map_postgres_err()?;
                            let before_recharge: f64 = row.try_get("balance").map_postgres_err()?;
                            let before_gift: f64 =
                                row.try_get("gift_balance").map_postgres_err()?;
                            let before_total = before_recharge + before_gift;
                            settlement.wallet_id = Some(wallet_id);
                            settlement.wallet_balance_before = Some(before_total);
                            settlement.wallet_balance_after = Some(before_total);
                            settlement.wallet_recharge_balance_before = Some(before_recharge);
                            settlement.wallet_recharge_balance_after = Some(before_recharge);
                            settlement.wallet_gift_balance_before = Some(before_gift);
                            settlement.wallet_gift_balance_after = Some(before_gift);
                        }

                        let wallet_debit_cost_usd = if !api_key_is_standalone {
                            if let Some(user_id) =
                                input.user_id.as_deref().filter(|value| !value.is_empty())
                            {
                                let sales_multiplier = settlement_wallet_charge_multiplier(&input);
                                let quota = consume_daily_quota_postgres(
                                    tx,
                                    DailyQuotaDebitInput {
                                        user_id,
                                        request_id: &input.request_id,
                                        total_cost_usd: input.base_cost_usd,
                                        wallet_available_usd,
                                        wallet_can_overdraft,
                                        wallet_charge_multiplier: sales_multiplier,
                                        request_global_model_id: input.global_model_id.as_deref(),
                                    },
                                )
                                .await?;
                                if quota.insufficient {
                                    final_billing_status = "insufficient_quota".to_string();
                                    settlement.billing_status = final_billing_status.clone();
                                    0.0
                                } else {
                                    (input.base_cost_usd - quota.debited_usd).max(0.0)
                                        * sales_multiplier
                                }
                            } else {
                                input.total_cost_usd
                            }
                        } else {
                            input.total_cost_usd
                        };
                        if final_billing_status != "settled" {
                            sync_usage_settlement_snapshot(&mut **tx, &settlement).await?;
                            sqlx::query(FINALIZE_USAGE_BILLING_SQL)
                                .bind(&input.request_id)
                                .bind(&final_billing_status)
                                .bind(finalized_at)
                                .execute(&mut **tx)
                                .await
                                .map_postgres_err()?;
                            return Ok(Some(settlement));
                        }

                        if wallet_debit_cost_usd > SETTLEMENT_EPSILON_USD {
                            if let Some(wallet_row) = wallet_row {
                                let wallet_id: String =
                                    wallet_row.try_get("id").map_postgres_err()?;
                                let before_recharge: f64 =
                                    wallet_row.try_get("balance").map_postgres_err()?;
                                let before_gift: f64 =
                                    wallet_row.try_get("gift_balance").map_postgres_err()?;
                                let limit_mode: String =
                                    wallet_row.try_get("limit_mode").map_postgres_err()?;
                                let before_total = before_recharge + before_gift;
                                let mut after_recharge = before_recharge;
                                let mut after_gift = before_gift;
                                if !limit_mode.eq_ignore_ascii_case("unlimited") {
                                    let debit_plan = plan_finite_wallet_debit(
                                        before_recharge,
                                        before_gift,
                                        wallet_debit_cost_usd,
                                    );
                                    (after_recharge, after_gift) =
                                        debit_plan.after_balances(before_recharge, before_gift);
                                }
                                if final_billing_status == "settled" {
                                    sqlx::query(
                                        r#"
UPDATE wallets
SET
  balance = $2,
  gift_balance = $3,
  total_consumed = CAST(total_consumed AS DOUBLE PRECISION) + $4,
  updated_at = NOW()
WHERE id = $1
                                "#,
                                    )
                                    .bind(&wallet_id)
                                    .bind(after_recharge)
                                    .bind(after_gift)
                                    .bind(wallet_debit_cost_usd)
                                    .execute(&mut **tx)
                                    .await
                                    .map_postgres_err()?;
                                }

                                settlement.wallet_id = Some(wallet_id.clone());
                                settlement.wallet_balance_before = Some(before_total);
                                settlement.wallet_balance_after = Some(after_recharge + after_gift);
                                settlement.wallet_recharge_balance_before = Some(before_recharge);
                                settlement.wallet_recharge_balance_after = Some(after_recharge);
                                settlement.wallet_gift_balance_before = Some(before_gift);
                                settlement.wallet_gift_balance_after = Some(after_gift);
                            } else {
                                final_billing_status = "insufficient_quota".to_string();
                                settlement.billing_status = final_billing_status.clone();
                            }
                        }

                        if final_billing_status != "settled" {
                            sync_usage_settlement_snapshot(&mut **tx, &settlement).await?;
                            sqlx::query(FINALIZE_USAGE_BILLING_SQL)
                                .bind(&input.request_id)
                                .bind(&final_billing_status)
                                .bind(finalized_at)
                                .execute(&mut **tx)
                                .await
                                .map_postgres_err()?;
                            return Ok(Some(settlement));
                        }

                        if let Some(provider_id) = input
                            .provider_id
                            .as_deref()
                            .filter(|value| !value.is_empty())
                        {
                            enqueue_provider_monthly_usage_delta(
                                &mut **tx,
                                &input.request_id,
                                provider_id,
                                input.actual_total_cost_usd,
                            )
                            .await?;
                        }
                    }

                    sync_usage_settlement_snapshot(&mut **tx, &settlement).await?;
                    sqlx::query(FINALIZE_USAGE_BILLING_SQL)
                        .bind(&input.request_id)
                        .bind(&final_billing_status)
                        .bind(finalized_at)
                        .execute(&mut **tx)
                        .await
                        .map_postgres_err()?;

                    Ok(Some(settlement))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn finalize_usage_billing_sql_does_not_require_usage_updated_at_column() {
        assert!(!super::FINALIZE_USAGE_BILLING_SQL.contains("updated_at"));
    }

    #[test]
    fn settlement_sql_reads_settlement_snapshots_before_legacy_usage_columns() {
        assert!(
            super::FIND_USAGE_FOR_SETTLEMENT_SQL.contains("LEFT JOIN usage_settlement_snapshots")
        );
        assert!(super::FIND_USAGE_FOR_SETTLEMENT_SQL.contains(
            "COALESCE(usage_settlement_snapshots.billing_status, usage_record.billing_status)"
        ));
        assert!(super::FIND_USAGE_FOR_SETTLEMENT_SQL.contains("FOR UPDATE OF usage_record"));
    }

    #[test]
    fn settlement_sql_dual_writes_usage_settlement_snapshots() {
        assert!(super::UPSERT_USAGE_SETTLEMENT_SNAPSHOT_SQL
            .contains("INSERT INTO usage_settlement_snapshots"));
        assert!(super::UPSERT_USAGE_SETTLEMENT_SNAPSHOT_SQL.contains("provider_monthly_used_usd"));
        assert!(super::UPSERT_USAGE_SETTLEMENT_SNAPSHOT_SQL
            .contains("TO_TIMESTAMP($11::double precision)"));
    }

    #[test]
    fn settlement_sql_no_longer_dual_writes_wallet_snapshots_to_usage_rows() {
        let source = include_str!("postgres.rs");
        assert!(!source.contains("UPDATE \"usage\"\nSET\n  wallet_id = $2"));
    }

    #[test]
    fn settlement_sql_enqueues_provider_monthly_usage_delta() {
        let source = include_str!("postgres.rs");
        assert!(super::ENQUEUE_PROVIDER_MONTHLY_USAGE_DELTA_SQL.contains("usage_counter_deltas"));
        assert!(super::ENQUEUE_PROVIDER_MONTHLY_USAGE_DELTA_SQL.contains("'provider_monthly'"));
        assert!(!source.contains("UPDATE providers\nSET\n  monthly_used_usd"));
    }

    #[test]
    fn settlement_sql_blocks_standalone_key_owner_wallet_fallback() {
        let source = include_str!("postgres.rs");
        assert!(source.contains("SELECT is_standalone"));
        assert!(source.contains("} else if !api_key_is_standalone {"));
    }
}
