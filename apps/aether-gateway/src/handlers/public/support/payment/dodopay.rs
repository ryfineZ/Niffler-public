use axum::{body::Body, http, response::Response};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use tracing::warn;
use uuid::Uuid;

use super::{payment_shared::payment_callback_payload_hash, AppState, GatewayPublicRequestContext};

#[derive(Debug, Clone)]
pub(crate) struct DodopayConfig {
    pub(crate) base_url: String,
    pub(crate) app_id: String,
    pub(crate) app_secret: String,
    pub(crate) callback_base_url: Option<String>,
    pub(crate) return_path: String,
    pub(crate) pay_currency: String,
    pub(crate) usd_exchange_rate: f64,
    pub(crate) min_recharge_usd: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct DodopayCheckoutInput {
    pub(crate) order_no: String,
    pub(crate) subject: String,
    pub(crate) pay_amount: f64,
    pub(crate) notify_url: String,
    pub(crate) return_url: String,
    pub(crate) metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub(crate) struct DodopayCheckoutOutput {
    pub(crate) gateway_order_id: String,
    pub(crate) pay_amount: f64,
    pub(crate) payment_instructions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DodopayCreateOrderResponse {
    order_id: String,
    amount: String,
    payable_amount: String,
    status: String,
    expires_at: Option<String>,
    checkout_url: String,
}

fn normalize_base_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    let parsed = url::Url::parse(trimmed).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return None;
    }
    Some(trimmed.to_string())
}

fn forwarded_header_first(value: String) -> Option<String> {
    value
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) async fn load_dodopay_config(state: &AppState) -> Result<DodopayConfig, String> {
    let Some(record) = state
        .find_payment_gateway_config("dodopay")
        .await
        .map_err(|err| format!("dodopay config lookup failed: {err:?}"))?
    else {
        return Err("DoDoPay 未配置".to_string());
    };
    if !record.enabled {
        return Err("DoDoPay 未启用".to_string());
    }
    let Some(encrypted_secret) = record.merchant_key_encrypted.as_deref() else {
        return Err("DoDoPay 应用密钥未配置".to_string());
    };
    let Some(app_secret) = crate::handlers::shared::decrypt_catalog_secret_with_fallbacks(
        state.encryption_key(),
        encrypted_secret,
    ) else {
        return Err("DoDoPay 应用密钥解密失败".to_string());
    };
    let app_id = record.merchant_id.trim();
    if app_id.is_empty() {
        return Err("DoDoPay 应用 ID 未配置".to_string());
    }
    let Some(base_url) = normalize_base_url(&record.endpoint_url) else {
        return Err("DoDoPay 服务地址必须是 http(s) 绝对地址".to_string());
    };
    let callback_base_url = record.callback_base_url;
    if let Some(value) = callback_base_url.as_deref() {
        if normalize_base_url(value).is_none() {
            return Err("DoDoPay 回调站点根地址必须是 http(s) 绝对地址".to_string());
        }
    }
    Ok(DodopayConfig {
        base_url,
        app_id: app_id.to_string(),
        app_secret,
        callback_base_url,
        return_path: "/dashboard/wallet".to_string(),
        pay_currency: record.pay_currency,
        usd_exchange_rate: record.usd_exchange_rate,
        min_recharge_usd: record.min_recharge_usd,
    })
}

pub(crate) fn dodopay_callback_base_url(
    configured: Option<&str>,
    headers: &http::HeaderMap,
    request_context: &GatewayPublicRequestContext,
) -> Option<String> {
    if let Some(value) = configured.and_then(normalize_base_url) {
        return Some(value);
    }

    if let Some(value) = std::env::var("AETHER_PUBLIC_BASE_URL")
        .ok()
        .or_else(|| std::env::var("PUBLIC_BASE_URL").ok())
        .and_then(|value| normalize_base_url(&value))
    {
        return Some(value);
    }

    let host = crate::headers::header_value_str(headers, crate::constants::FORWARDED_HOST_HEADER)
        .and_then(forwarded_header_first)
        .or_else(|| request_context.host_header.clone())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| {
            !value.is_empty()
                && !value.contains('/')
                && !value.contains('\\')
                && !value.contains('@')
                && !value.contains(char::is_whitespace)
        })?;
    let proto = crate::headers::header_value_str(headers, crate::constants::FORWARDED_PROTO_HEADER)
        .and_then(forwarded_header_first)
        .map(|value| value.trim().trim_end_matches(':').to_ascii_lowercase())
        .filter(|value| value == "http" || value == "https")
        .unwrap_or_else(|| "http".to_string());
    normalize_base_url(&format!("{proto}://{host}"))
}

fn normalize_return_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

pub(crate) fn dodopay_return_url(config: &DodopayConfig, callback_base_url: &str) -> String {
    format!(
        "{}{}",
        callback_base_url.trim_end_matches('/'),
        normalize_return_path(&config.return_path)
    )
}

fn dodopay_canonicalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut items = map.iter().collect::<Vec<_>>();
            items.sort_by(|left, right| left.0.cmp(right.0));
            let mut object = serde_json::Map::new();
            for (key, value) in items {
                object.insert(key.clone(), dodopay_canonicalize_json(value));
            }
            serde_json::Value::Object(object)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(dodopay_canonicalize_json).collect())
        }
        _ => value.clone(),
    }
}

fn dodopay_unsigned_payload(payload: &serde_json::Value) -> serde_json::Value {
    let mut unsigned = payload.clone();
    if let serde_json::Value::Object(object) = &mut unsigned {
        object.remove("signature");
    }
    unsigned
}

pub(crate) fn dodopay_sign_payload(
    app_secret: &str,
    payload: &serde_json::Value,
) -> Result<String, String> {
    let canonical = dodopay_canonicalize_json(&dodopay_unsigned_payload(payload));
    let encoded = serde_json::to_string(&canonical)
        .map_err(|err| format!("dodopay payload encode failed: {err}"))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes())
        .map_err(|err| format!("dodopay hmac init failed: {err}"))?;
    mac.update(encoded.as_bytes());
    let bytes = mac.finalize().into_bytes();
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn dodopay_timing_safe_equal(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .fold(0u8, |acc, (left, right)| acc | (left ^ right))
            == 0
}

pub(crate) fn dodopay_verify_payload_signature(
    app_secret: &str,
    payload: &serde_json::Value,
) -> Result<bool, String> {
    let provided = payload
        .get("signature")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .unwrap_or("")
        .to_ascii_lowercase();
    if provided.is_empty() {
        return Ok(false);
    }
    let expected = dodopay_sign_payload(app_secret, payload)?;
    Ok(dodopay_timing_safe_equal(&provided, &expected))
}

fn dodopay_order_url(config: &DodopayConfig) -> String {
    format!("{}/api/v1/orders", config.base_url.trim_end_matches('/'))
}

fn parse_decimal_amount(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| "dodopay amount parse failed".to_string())
        .and_then(|amount| {
            if amount.is_finite() && amount > 0.0 {
                Ok(amount)
            } else {
                Err("dodopay amount is invalid".to_string())
            }
        })
}

pub(crate) async fn create_dodopay_checkout(
    config: &DodopayConfig,
    input: &DodopayCheckoutInput,
) -> Result<DodopayCheckoutOutput, String> {
    let unsigned = json!({
        "app_id": config.app_id,
        "merchant_order_id": input.order_no,
        "amount": format!("{:.2}", input.pay_amount),
        "subject": input.subject,
        "notify_url": input.notify_url,
        "return_url": input.return_url,
        "metadata": input.metadata,
        "nonce": Uuid::new_v4().simple().to_string(),
        "timestamp": Utc::now().timestamp(),
    });
    let signature = dodopay_sign_payload(&config.app_secret, &unsigned)?;
    let mut body = unsigned;
    body["signature"] = json!(signature);

    let response = reqwest::Client::new()
        .post(dodopay_order_url(config))
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("dodopay create order failed: {err}"))?;
    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|err| format!("dodopay response read failed: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "dodopay create order returned {status}: {response_text}"
        ));
    }
    let order: DodopayCreateOrderResponse = serde_json::from_str(&response_text)
        .map_err(|err| format!("dodopay response parse failed: {err}"))?;
    let pay_amount = parse_decimal_amount(&order.payable_amount)
        .or_else(|_| parse_decimal_amount(&order.amount))
        .unwrap_or(input.pay_amount);
    let payment_instructions = json!({
        "gateway": "dodopay",
        "display_name": "DoDoPay",
        "gateway_order_id": order.order_id,
        "payment_url": order.checkout_url,
        "submit_method": "GET",
        "qr_code": serde_json::Value::Null,
        "pay_amount": pay_amount,
        "pay_currency": config.pay_currency,
        "payment_channel": serde_json::Value::Null,
        "provider_order_status": order.status,
        "expires_at": order.expires_at,
    });

    Ok(DodopayCheckoutOutput {
        gateway_order_id: order.order_id,
        pay_amount,
        payment_instructions,
    })
}

fn dodopay_plain(status: http::StatusCode, body: &'static str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .expect("dodopay plain response should build")
}

fn dodopay_redirect(location: String) -> Response<Body> {
    Response::builder()
        .status(http::StatusCode::FOUND)
        .header(http::header::LOCATION, location)
        .body(Body::empty())
        .expect("dodopay redirect response should build")
}

fn dodopay_return_location(query: Option<&str>) -> String {
    let suffix = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("?{value}"))
        .unwrap_or_default();
    format!("/dashboard/wallet{suffix}")
}

pub(super) async fn handle_dodopay_return(
    request_context: &GatewayPublicRequestContext,
) -> Response<Body> {
    dodopay_redirect(dodopay_return_location(
        request_context.request_query_string.as_deref(),
    ))
}

pub(super) async fn handle_dodopay_notify(
    state: &AppState,
    request_body: Option<&axum::body::Bytes>,
) -> Response<Body> {
    let config = match load_dodopay_config(state).await {
        Ok(value) => value,
        Err(_) => return dodopay_plain(http::StatusCode::SERVICE_UNAVAILABLE, "fail"),
    };
    let Some(request_body) = request_body else {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    };
    let payload: serde_json::Value = match serde_json::from_slice(request_body) {
        Ok(value) => value,
        Err(_) => return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail"),
    };
    let signature_valid =
        dodopay_verify_payload_signature(&config.app_secret, &payload).unwrap_or_default();
    if !signature_valid {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    }
    if payload.get("event_type").and_then(|value| value.as_str()) != Some("payment.succeeded") {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    }
    if payload.get("app_id").and_then(|value| value.as_str()) != Some(config.app_id.as_str()) {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    }
    let Some(order_no) = payload
        .get("merchant_order_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
    else {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    };
    let Some(gateway_order_id) = payload
        .get("order_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
    else {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    };
    let Some(pay_amount) = payload
        .get("payable_amount")
        .and_then(|value| value.as_str())
        .and_then(|value| parse_decimal_amount(value).ok())
    else {
        return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail");
    };
    let amount_usd = if config.usd_exchange_rate > 0.0 {
        pay_amount / config.usd_exchange_rate
    } else {
        pay_amount
    };
    let channel = payload
        .get("channel")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let payload_hash = match payment_callback_payload_hash(&payload) {
        Ok(value) => value,
        Err(_) => return dodopay_plain(http::StatusCode::BAD_REQUEST, "fail"),
    };
    let callback_key = payload
        .get("event_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("dodopay:{gateway_order_id}:{payload_hash}"));

    let outcome = state
        .process_payment_callback(
            aether_data::repository::wallet::ProcessPaymentCallbackInput {
                payment_method: "dodopay".to_string(),
                payment_provider: Some("dodopay".to_string()),
                payment_channel: channel,
                callback_key,
                order_no: Some(order_no),
                gateway_order_id: Some(gateway_order_id),
                amount_usd,
                pay_amount: Some(pay_amount),
                pay_currency: Some(config.pay_currency),
                exchange_rate: Some(config.usd_exchange_rate),
                payload_hash,
                payload,
                signature_valid: true,
            },
        )
        .await;

    match outcome {
        Ok(Some(aether_data::repository::wallet::ProcessPaymentCallbackOutcome::Applied {
            order,
            order_id,
            ..
        })) => {
            if let Err(err) = state.apply_referral_rewards_for_paid_order(&order).await {
                warn!(
                    error = ?err,
                    order_id = %order_id,
                    "failed to apply referral rewards for dodopay callback"
                );
            }
            dodopay_plain(http::StatusCode::OK, "success")
        }
        Ok(Some(
            aether_data::repository::wallet::ProcessPaymentCallbackOutcome::AlreadyCredited {
                ..
            },
        ))
        | Ok(Some(
            aether_data::repository::wallet::ProcessPaymentCallbackOutcome::DuplicateProcessed {
                ..
            },
        )) => dodopay_plain(http::StatusCode::OK, "success"),
        _ => dodopay_plain(http::StatusCode::INTERNAL_SERVER_ERROR, "fail"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn dodopay_signs_stable_json_without_signature() {
        let mut payload = json!({
            "timestamp": 1710000000,
            "nonce": "nonce-123456",
            "app_id": "app_test",
            "merchant_order_id": "po_test",
            "amount": "9.90",
            "subject": "钱包充值",
            "metadata": {
                "signature": "kept"
            }
        });
        let first = super::dodopay_sign_payload("secret", &payload).expect("sign should work");
        let second = super::dodopay_sign_payload("secret", &payload).expect("sign should work");
        assert_eq!(first, second);

        payload["signature"] = json!("ignored");
        let with_signature =
            super::dodopay_sign_payload("secret", &payload).expect("sign should work");
        assert_eq!(first, with_signature);

        payload["metadata"]["signature"] = json!("still-signed");
        let with_nested_signature_changed =
            super::dodopay_sign_payload("secret", &payload).expect("sign should work");
        assert_ne!(first, with_nested_signature_changed);
    }

    #[test]
    fn dodopay_callback_signature_requires_matching_secret() {
        let mut payload = json!({
            "event_id": "evt_1",
            "event_type": "payment.succeeded",
            "app_id": "app_test",
            "order_id": "order_1",
            "merchant_order_id": "po_test",
            "amount": "9.90",
            "payable_amount": "9.91",
            "channel": "ALIPAY",
            "paid_at": "2026-05-26T10:08:00.000Z",
            "metadata": null,
            "timestamp": 1710000000
        });
        let signature =
            super::dodopay_sign_payload("secret", &payload).expect("signature should build");
        payload["signature"] = json!(signature);

        assert!(super::dodopay_verify_payload_signature("secret", &payload)
            .expect("verification should work"));
        assert!(!super::dodopay_verify_payload_signature("wrong", &payload)
            .expect("verification should work"));
    }

    #[tokio::test]
    async fn dodopay_notify_is_disabled_without_gateway_config() {
        let state = super::AppState::new().expect("state should build");
        let body = axum::body::Bytes::from(
            serde_json::to_vec(&json!({
                "event_id": "evt_1",
                "event_type": "payment.succeeded",
                "app_id": "app_test",
                "order_id": "order_1",
                "merchant_order_id": "po_test",
                "amount": "9.90",
                "payable_amount": "9.91",
                "channel": "ALIPAY",
                "paid_at": "2026-05-26T10:08:00.000Z",
                "metadata": null,
                "timestamp": 1710000000
            }))
            .expect("payload should encode"),
        );

        let response = super::handle_dodopay_notify(&state, Some(&body)).await;

        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
