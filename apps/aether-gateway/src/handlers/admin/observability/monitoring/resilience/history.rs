use super::snapshot::build_admin_monitoring_provider_name_by_id_and_keys;
use crate::handlers::admin::request::{AdminAppState, AdminRequestContext};
use crate::handlers::shared::{
    parse_catalog_auth_config_json, provider_key_account_label_from_auth_config,
};
use crate::GatewayError;
use aether_admin::observability::monitoring::{
    admin_monitoring_bad_request_response, build_admin_monitoring_circuit_history_items,
    build_admin_monitoring_circuit_history_payload_response,
    parse_admin_monitoring_circuit_history_limit,
};
use axum::{body::Body, response::Response};
use serde_json::json;
use std::collections::BTreeMap;

pub(in super::super) async fn build_admin_monitoring_resilience_circuit_history_response(
    state: &AdminAppState<'_>,
    request_context: &AdminRequestContext<'_>,
) -> Result<Response<Body>, GatewayError> {
    let limit = match parse_admin_monitoring_circuit_history_limit(
        request_context.request_query_string.as_deref(),
    ) {
        Ok(value) => value,
        Err(detail) => return Ok(admin_monitoring_bad_request_response(detail)),
    };

    let (provider_name_by_id, keys) =
        build_admin_monitoring_provider_name_by_id_and_keys(state).await?;
    let account_labels: BTreeMap<_, _> = keys
        .iter()
        .filter_map(|key| {
            let label = provider_key_account_label_from_auth_config(
                parse_catalog_auth_config_json(state.app(), key).as_ref(),
            )?;
            Some((key.id.clone(), label))
        })
        .collect();
    let mut items =
        build_admin_monitoring_circuit_history_items(&keys, &provider_name_by_id, limit);
    for item in &mut items {
        let Some(key_id) = item.get("key_id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if let Some(label) = account_labels.get(key_id) {
            item["key_account_label"] = json!(label);
        }
    }
    Ok(build_admin_monitoring_circuit_history_payload_response(
        items,
    ))
}
