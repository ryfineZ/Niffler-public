use super::{
    build_auth_error_response, build_auth_json_response, query_param_value, support_wallet,
    AppState, Body, GatewayPublicRequestContext,
};
use crate::control::GatewayLocalAuthRejection;
use axum::{http, response::Response};
use serde_json::{json, Value};

pub(super) async fn maybe_build_local_ccswitch_usage_response(
    state: &AppState,
    request_context: &GatewayPublicRequestContext,
) -> Option<Response<Body>> {
    let decision = request_context.control_decision.as_ref()?;
    if decision.route_family.as_deref() != Some("ccswitch")
        || decision.route_kind.as_deref() != Some("usage")
        || request_context.request_path != "/v1/usage"
    {
        return None;
    }

    Some(handle_ccswitch_usage(state, request_context).await)
}

async fn handle_ccswitch_usage(
    state: &AppState,
    request_context: &GatewayPublicRequestContext,
) -> Response<Body> {
    let Some(auth_context) = request_context
        .control_decision
        .as_ref()
        .and_then(|decision| decision.auth_context.as_ref())
    else {
        return build_auth_error_response(http::StatusCode::UNAUTHORIZED, "Invalid API key", false);
    };

    if matches!(
        auth_context.local_rejection.as_ref(),
        Some(GatewayLocalAuthRejection::InvalidApiKey | GatewayLocalAuthRejection::LockedApiKey)
    ) {
        return build_auth_json_response(
            http::StatusCode::OK,
            json!({
                "is_active": false,
                "isValid": false,
                "unit": "USD",
                "remaining": 0.0,
                "balance": 0.0,
                "api_key": {
                    "id": auth_context.api_key_id,
                    "name": auth_context.api_key_name,
                },
            }),
            None,
        );
    }

    let wallet = state
        .read_wallet_snapshot_for_auth(
            &auth_context.user_id,
            &auth_context.api_key_id,
            auth_context.api_key_is_standalone,
        )
        .await
        .ok()
        .flatten();
    let model_filter = query_param_value(request_context.request_query_string.as_deref(), "model")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let wallet_payload = support_wallet::build_wallet_balance_payload_for_user_and_model(
        state,
        &auth_context.user_id,
        wallet.as_ref(),
        model_filter.as_deref(),
    )
    .await;

    let currency = wallet_payload
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("USD");
    let wallet_balance = number_value(&wallet_payload, "wallet_balance").unwrap_or(0.0);
    let package_balance = number_value(&wallet_payload, "package_balance").unwrap_or(0.0);
    let total_available_balance = wallet_payload
        .get("total_available_balance")
        .cloned()
        .unwrap_or_else(|| json!(null));
    let remaining = total_available_balance
        .as_f64()
        .or(auth_context.balance_remaining)
        .unwrap_or(wallet_balance + package_balance);

    build_auth_json_response(
        http::StatusCode::OK,
        json!({
            "is_active": true,
            "isValid": true,
            "unit": currency,
            "remaining": remaining,
            "balance": remaining,
            "wallet_balance": wallet_balance,
            "package_balance": package_balance,
            "total_available_balance": total_available_balance,
            "unlimited": wallet_payload
                .get("unlimited")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "daily_quota": wallet_payload.get("daily_quota").cloned().unwrap_or_else(|| json!(null)),
            "deduction_order": wallet_payload
                .get("deduction_order")
                .cloned()
                .unwrap_or_else(|| json!([])),
            "quota": {
                "unit": currency,
                "remaining": remaining,
                "wallet_balance": wallet_balance,
                "package_balance": package_balance,
                "daily_quota": wallet_payload.get("daily_quota").cloned().unwrap_or_else(|| json!(null)),
            },
            "api_key": {
                "id": auth_context.api_key_id,
                "name": auth_context.api_key_name,
            },
            "model": model_filter,
        }),
        None,
    )
}

fn number_value(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key).and_then(Value::as_f64)
}
