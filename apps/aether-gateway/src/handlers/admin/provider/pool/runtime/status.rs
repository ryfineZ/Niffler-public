use super::reads::read_admin_provider_pool_runtime_state;
use crate::handlers::admin::provider::pool::config::admin_provider_pool_config;
use crate::handlers::admin::request::AdminAppState;
use serde_json::json;

fn pool_status_auth_config_string(
    auth_config: Option<&serde_json::Map<String, serde_json::Value>>,
    fields: &[&str],
) -> serde_json::Value {
    for field in fields {
        let Some(value) = auth_config
            .and_then(|config| config.get(*field))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        return json!(value);
    }
    serde_json::Value::Null
}

pub(crate) async fn build_admin_provider_pool_status_payload(
    state: &AdminAppState<'_>,
    provider_id: &str,
) -> Option<serde_json::Value> {
    if !state.has_provider_catalog_data_reader() {
        return None;
    }

    let provider = state
        .read_provider_catalog_providers_by_ids(&[provider_id.to_string()])
        .await
        .ok()
        .and_then(|mut providers| providers.drain(..).next())?;
    let Some(pool_config) = admin_provider_pool_config(&provider) else {
        return Some(json!({
            "provider_id": provider.id,
            "provider_name": provider.name,
            "pool_enabled": false,
            "total_keys": 0,
            "total_sticky_sessions": 0,
            "provider_hot_count": 0,
            "provider_desired_hot": 0,
            "provider_in_flight": 0,
            "provider_ema_in_flight": 0.0,
            "provider_burst_pending": false,
            "keys": [],
        }));
    };

    let keys = state
        .list_provider_catalog_keys_by_provider_ids(std::slice::from_ref(&provider.id))
        .await
        .ok()
        .unwrap_or_default();
    let key_ids = keys.iter().map(|key| key.id.clone()).collect::<Vec<_>>();
    let runtime = read_admin_provider_pool_runtime_state(
        state.runtime_state(),
        &provider.id,
        &key_ids,
        &pool_config,
        None,
    )
    .await;
    let key_payloads = keys
        .into_iter()
        .map(|key| {
            let cooldown_reason = runtime.cooldown_reason_by_key.get(&key.id).cloned();
            let auth_config = state.parse_catalog_auth_config_json(&key);
            json!({
                "key_id": key.id,
                "key_name": key.name,
                "oauth_email": pool_status_auth_config_string(auth_config.as_ref(), &["email", "oauth_email"]),
                "oauth_phone": pool_status_auth_config_string(
                    auth_config.as_ref(),
                    &["phone", "phone_number", "mobile", "mobile_phone"],
                ),
                "is_active": key.is_active,
                "cooldown_reason": cooldown_reason,
                "cooldown_ttl_seconds": cooldown_reason
                    .as_ref()
                    .and_then(|_| runtime.cooldown_ttl_by_key.get(&key.id).copied()),
                "cost_window_usage": runtime.cost_window_usage_by_key.get(&key.id).copied().unwrap_or(0),
                "cost_limit": pool_config.cost_limit_per_key_tokens,
                "sticky_sessions": runtime.sticky_sessions_by_key.get(&key.id).copied().unwrap_or(0),
                "latency_avg_ms": runtime.latency_avg_ms_by_key.get(&key.id).copied(),
                "lru_score": runtime.lru_score_by_key.get(&key.id).copied(),
            })
        })
        .collect::<Vec<_>>();

    Some(json!({
        "provider_id": provider.id,
        "provider_name": provider.name,
        "pool_enabled": true,
        "total_keys": key_payloads.len(),
        "total_sticky_sessions": runtime.total_sticky_sessions,
        "provider_hot_count": runtime.active_probe_member_ids.len(),
        "provider_desired_hot": runtime.provider_desired_hot,
        "provider_in_flight": runtime.provider_in_flight,
        "provider_ema_in_flight": runtime.provider_ema_in_flight,
        "provider_burst_pending": runtime.provider_burst_pending,
        "keys": key_payloads,
    }))
}
