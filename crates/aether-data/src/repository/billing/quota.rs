use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use crate::DataLayerError;

pub(crate) const QUOTA_SCOPE_DAILY: &str = "daily";
pub(crate) const QUOTA_SCOPE_FIVE_HOUR: &str = "five_hour";
pub(crate) const QUOTA_SCOPE_WEEKLY: &str = "weekly";
pub(crate) const QUOTA_SCOPE_MONTHLY: &str = "monthly";

#[derive(Debug, Clone)]
pub(crate) struct UsageQuotaGrant {
    pub entitlement_id: String,
    pub scope: &'static str,
    pub limit_usd: f64,
    pub window_key: String,
    pub window_started_at: DateTime<Utc>,
    pub window_ends_at: DateTime<Utc>,
    pub allow_wallet_overage: bool,
    pub allowed_global_model_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(crate) struct UsageQuotaWindow {
    pub scope: &'static str,
    pub key: String,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredUsageQuotaWindow {
    pub window_key: String,
    pub window_started_at: DateTime<Utc>,
    pub window_ends_at: DateTime<Utc>,
}

pub(crate) fn usage_quota_grants_from_entitlement(
    entitlement_id: &str,
    entitlements: &Value,
    now: DateTime<Utc>,
    entitlement_started_at: DateTime<Utc>,
    stored_five_hour: Option<&StoredUsageQuotaWindow>,
) -> Result<Vec<UsageQuotaGrant>, DataLayerError> {
    let mut grants = Vec::new();
    for item in usage_quota_items(entitlements) {
        let allow_wallet_overage = item
            .get("allow_wallet_overage")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allowed_global_model_ids = parse_allowed_global_model_ids(item);
        let limits = item.get("limits");

        push_grant(
            &mut grants,
            entitlement_id,
            quota_value(item, limits, "daily_quota_usd", "daily_limit_usd"),
            rolling_window(
                now,
                entitlement_started_at,
                Duration::days(1),
                QUOTA_SCOPE_DAILY,
            ),
            allow_wallet_overage,
            allowed_global_model_ids.as_deref(),
        );
        push_grant(
            &mut grants,
            entitlement_id,
            quota_value(item, limits, "five_hour_quota_usd", "five_hour_limit_usd"),
            five_hour_window(now, stored_five_hour),
            allow_wallet_overage,
            allowed_global_model_ids.as_deref(),
        );
        push_grant(
            &mut grants,
            entitlement_id,
            quota_value(item, limits, "weekly_quota_usd", "weekly_limit_usd"),
            rolling_window(
                now,
                entitlement_started_at,
                Duration::weeks(1),
                QUOTA_SCOPE_WEEKLY,
            ),
            allow_wallet_overage,
            allowed_global_model_ids.as_deref(),
        );
        push_grant(
            &mut grants,
            entitlement_id,
            quota_value(item, limits, "monthly_quota_usd", "monthly_limit_usd"),
            rolling_window(
                now,
                entitlement_started_at,
                Duration::days(30),
                QUOTA_SCOPE_MONTHLY,
            ),
            allow_wallet_overage,
            allowed_global_model_ids.as_deref(),
        );
    }
    Ok(grants)
}

pub(crate) fn entitlements_snapshot_has_usage_quota_for_global_model(
    entitlements: &Value,
    global_model_id: Option<&str>,
) -> bool {
    usage_quota_items(entitlements).into_iter().any(|item| {
        let allowed_global_model_ids = parse_allowed_global_model_ids(item);
        entitlement_allows_global_model(allowed_global_model_ids.as_deref(), global_model_id)
            && usage_quota_item_has_positive_limit(item)
    })
}

pub(crate) fn entitlement_allows_global_model(
    allowed_global_model_ids: Option<&[String]>,
    request_global_model_id: Option<&str>,
) -> bool {
    let Some(allowed_global_model_ids) = allowed_global_model_ids else {
        return true;
    };
    if allowed_global_model_ids.is_empty() {
        return true;
    }
    let Some(request_global_model_id) = request_global_model_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    allowed_global_model_ids
        .iter()
        .any(|id| id == request_global_model_id)
}

fn parse_allowed_global_model_ids(item: &Value) -> Option<Vec<String>> {
    item.get("allowed_global_model_ids")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
}

fn usage_quota_items(entitlements: &Value) -> Vec<&Value> {
    let items = if let Some(items) = entitlements.as_array() {
        items.iter().collect::<Vec<_>>()
    } else if entitlements.get("limits").is_some() {
        vec![entitlements]
    } else {
        Vec::new()
    };
    items
        .into_iter()
        .filter(|item| usage_quota_item_has_supported_shape(item))
        .collect()
}

fn usage_quota_item_has_supported_shape(item: &Value) -> bool {
    let entitlement_type = item.get("type").and_then(Value::as_str);
    matches!(entitlement_type, Some("daily_quota" | "usage_quota")) || item.get("limits").is_some()
}

fn usage_quota_item_has_positive_limit(item: &Value) -> bool {
    let limits = item.get("limits");
    [
        ("daily_quota_usd", "daily_limit_usd"),
        ("five_hour_quota_usd", "five_hour_limit_usd"),
        ("weekly_quota_usd", "weekly_limit_usd"),
        ("monthly_quota_usd", "monthly_limit_usd"),
    ]
    .into_iter()
    .any(|(standard_key, legacy_key)| quota_value(item, limits, standard_key, legacy_key) > 0.0)
}

fn quota_value(item: &Value, limits: Option<&Value>, standard_key: &str, legacy_key: &str) -> f64 {
    let standard = value_as_f64(item.get(standard_key));
    if standard > 0.0 {
        return standard;
    }
    let legacy_top_level = value_as_f64(item.get(legacy_key));
    if legacy_top_level > 0.0 {
        return legacy_top_level;
    }
    value_as_f64(limits.and_then(|limits| limits.get(legacy_key)))
}

fn value_as_f64(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn push_grant(
    grants: &mut Vec<UsageQuotaGrant>,
    entitlement_id: &str,
    limit_usd: f64,
    window: UsageQuotaWindow,
    allow_wallet_overage: bool,
    allowed_global_model_ids: Option<&[String]>,
) {
    if !limit_usd.is_finite() || limit_usd <= 0.0 {
        return;
    }
    grants.push(UsageQuotaGrant {
        entitlement_id: entitlement_id.to_string(),
        scope: window.scope,
        limit_usd,
        window_key: window.key,
        window_started_at: window.started_at,
        window_ends_at: window.ends_at,
        allow_wallet_overage,
        allowed_global_model_ids: allowed_global_model_ids.map(|items| items.to_vec()),
    });
}

fn five_hour_window(
    now: DateTime<Utc>,
    stored: Option<&StoredUsageQuotaWindow>,
) -> UsageQuotaWindow {
    if let Some(stored) = stored {
        if stored.window_ends_at > now {
            return UsageQuotaWindow {
                scope: QUOTA_SCOPE_FIVE_HOUR,
                key: stored.window_key.clone(),
                started_at: stored.window_started_at,
                ends_at: stored.window_ends_at,
            };
        }
    }
    let started_at = now;
    let ends_at = now + Duration::hours(5);
    UsageQuotaWindow {
        scope: QUOTA_SCOPE_FIVE_HOUR,
        key: format!("fh-{}", now.timestamp()),
        started_at,
        ends_at,
    }
}

fn rolling_window(
    now: DateTime<Utc>,
    entitlement_started_at: DateTime<Utc>,
    duration: Duration,
    scope: &'static str,
) -> UsageQuotaWindow {
    let duration_secs = duration.num_seconds().max(1);
    let elapsed_secs = (now - entitlement_started_at).num_seconds().max(0);
    let completed_windows = elapsed_secs / duration_secs;
    let started_at = entitlement_started_at + Duration::seconds(completed_windows * duration_secs);
    let ends_at = started_at + duration;
    let prefix = match scope {
        QUOTA_SCOPE_DAILY => "day",
        QUOTA_SCOPE_WEEKLY => "week",
        QUOTA_SCOPE_MONTHLY => "month",
        _ => "window",
    };
    UsageQuotaWindow {
        scope,
        key: format!("{prefix}-{}", started_at.timestamp()),
        started_at,
        ends_at,
    }
}
#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::{
        entitlement_allows_global_model, entitlements_snapshot_has_usage_quota_for_global_model,
        usage_quota_grants_from_entitlement, StoredUsageQuotaWindow, QUOTA_SCOPE_DAILY,
        QUOTA_SCOPE_FIVE_HOUR, QUOTA_SCOPE_MONTHLY, QUOTA_SCOPE_WEEKLY,
    };

    #[test]
    fn scoped_entitlement_allows_matching_global_model() {
        assert!(entitlement_allows_global_model(
            Some(&["global-codex".to_string()]),
            Some("global-codex"),
        ));
    }

    #[test]
    fn scoped_entitlement_rejects_other_global_model() {
        assert!(!entitlement_allows_global_model(
            Some(&["global-codex".to_string()]),
            Some("global-claude"),
        ));
    }

    #[test]
    fn scoped_entitlement_rejects_unknown_request_model() {
        assert!(!entitlement_allows_global_model(
            Some(&["global-codex".to_string()]),
            None,
        ));
    }

    #[test]
    fn unscoped_entitlement_keeps_legacy_behavior() {
        assert!(entitlement_allows_global_model(None, Some("global-claude")));
    }

    #[test]
    fn entitlement_snapshot_model_filter_ignores_unrelated_model_before_window_lookup() {
        let entitlements = json!([
            {
                "type": "daily_quota",
                "daily_quota_usd": 100.0,
                "allowed_global_model_ids": ["global-codex"]
            }
        ]);

        assert!(entitlements_snapshot_has_usage_quota_for_global_model(
            &entitlements,
            Some("global-codex")
        ));
        assert!(!entitlements_snapshot_has_usage_quota_for_global_model(
            &entitlements,
            Some("global-claude")
        ));
    }

    #[test]
    fn entitlement_snapshot_model_filter_keeps_unscoped_and_positive_legacy_limits() {
        let entitlements = json!({
            "limits": {
                "weekly_limit_usd": 25.0
            }
        });

        assert!(entitlements_snapshot_has_usage_quota_for_global_model(
            &entitlements,
            Some("global-claude")
        ));
    }

    #[test]
    fn entitlement_snapshot_model_filter_ignores_zero_limit_items() {
        let entitlements = json!([
            {
                "type": "daily_quota",
                "daily_quota_usd": 0.0,
                "allowed_global_model_ids": ["global-codex"]
            }
        ]);

        assert!(!entitlements_snapshot_has_usage_quota_for_global_model(
            &entitlements,
            Some("global-codex")
        ));
    }

    #[test]
    fn five_hour_window_reuses_active_user_window() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 5, 28, 10, 0, 0).unwrap();
        let started_at = chrono::Utc.with_ymd_and_hms(2026, 5, 28, 8, 0, 0).unwrap();
        let stored = StoredUsageQuotaWindow {
            window_key: "fh-existing".to_string(),
            window_started_at: started_at,
            window_ends_at: started_at + chrono::Duration::hours(5),
        };
        let grants = usage_quota_grants_from_entitlement(
            "ent-1",
            &json!([{"type":"daily_quota","five_hour_quota_usd":20.0}]),
            now,
            started_at,
            Some(&stored),
        )
        .expect("quota grants should parse");

        let grant = grants
            .iter()
            .find(|grant| grant.scope == QUOTA_SCOPE_FIVE_HOUR)
            .expect("five hour grant should exist");
        assert_eq!(grant.window_key, "fh-existing");
        assert_eq!(grant.window_started_at, started_at);
    }

    #[test]
    fn daily_weekly_and_monthly_windows_roll_from_entitlement_start() {
        let entitlement_started_at = chrono::Utc
            .with_ymd_and_hms(2026, 5, 27, 15, 30, 0)
            .unwrap();
        let now = chrono::Utc.with_ymd_and_hms(2026, 6, 3, 16, 0, 0).unwrap();
        let grants = usage_quota_grants_from_entitlement(
            "ent-1",
            &json!([{
                "type":"daily_quota",
                "daily_quota_usd":10.0,
                "weekly_quota_usd":20.0,
                "monthly_quota_usd":30.0
            }]),
            now,
            entitlement_started_at,
            None,
        )
        .expect("quota grants should parse");

        let daily = grants
            .iter()
            .find(|grant| grant.scope == QUOTA_SCOPE_DAILY)
            .expect("daily grant should exist");
        assert_eq!(
            daily.window_started_at,
            chrono::Utc.with_ymd_and_hms(2026, 6, 3, 15, 30, 0).unwrap()
        );
        assert_eq!(
            daily.window_ends_at,
            chrono::Utc.with_ymd_and_hms(2026, 6, 4, 15, 30, 0).unwrap()
        );

        let weekly = grants
            .iter()
            .find(|grant| grant.scope == QUOTA_SCOPE_WEEKLY)
            .expect("weekly grant should exist");
        assert_eq!(
            weekly.window_started_at,
            entitlement_started_at + chrono::Duration::weeks(1)
        );
        assert_eq!(
            weekly.window_ends_at,
            entitlement_started_at + chrono::Duration::weeks(2)
        );

        let monthly = grants
            .iter()
            .find(|grant| grant.scope == QUOTA_SCOPE_MONTHLY)
            .expect("monthly grant should exist");
        assert_eq!(monthly.window_started_at, entitlement_started_at);
        assert_eq!(
            monthly.window_ends_at,
            entitlement_started_at + chrono::Duration::days(30)
        );
    }
}
