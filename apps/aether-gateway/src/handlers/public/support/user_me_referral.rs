use super::{
    build_auth_error_response, resolve_authenticated_local_user, AppState,
    GatewayPublicRequestContext,
};
use axum::{
    body::Body,
    http,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::headers::header_value_str;

pub(super) async fn handle_users_me_referral_get(
    state: &AppState,
    request_context: &GatewayPublicRequestContext,
    headers: &http::HeaderMap,
) -> Response<Body> {
    let auth = match resolve_authenticated_local_user(state, request_context, headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !state.has_referral_data_backend() {
        return build_auth_error_response(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "邀请返利数据暂不可用",
            false,
        );
    }
    let dashboard = match state.referral_dashboard(&auth.user.id).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return build_auth_error_response(
                http::StatusCode::SERVICE_UNAVAILABLE,
                "邀请返利数据暂不可用",
                false,
            );
        }
        Err(err) => {
            return build_auth_error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("referral dashboard failed: {err:?}"),
                false,
            );
        }
    };
    let invitation_link = build_invitation_link(headers, dashboard.invite_code.as_str());
    Json(json!({
        "invite_code": dashboard.invite_code,
        "invitation_link": invitation_link,
        "summary": {
            "total_invites": dashboard.total_invites,
            "effective_invites": dashboard.effective_invites,
            "paid_reward_usd": dashboard.paid_reward_usd,
            "pending_reward_usd": dashboard.pending_reward_usd,
            "reversed_reward_usd": dashboard.reversed_reward_usd,
        }
    }))
    .into_response()
}

fn build_invitation_link(headers: &http::HeaderMap, invite_code: &str) -> String {
    let path = format!("/register?invite={invite_code}");
    if let Some(origin) = header_value_str(headers, http::header::ORIGIN.as_str()) {
        return format!("{}{}", origin.trim_end_matches('/'), path);
    }

    let host = header_value_str(headers, "x-forwarded-host")
        .or_else(|| header_value_str(headers, http::header::HOST.as_str()))
        .and_then(|value| {
            value
                .split(',')
                .map(str::trim)
                .find(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        });
    let Some(host) = host else {
        return path;
    };

    let proto = header_value_str(headers, "x-forwarded-proto")
        .and_then(|value| {
            value
                .split(',')
                .map(str::trim)
                .find(|value| matches!(*value, "http" | "https"))
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
                "http".to_string()
            } else {
                "https".to_string()
            }
        });

    format!("{proto}://{host}{path}")
}

#[cfg(test)]
mod tests {
    use super::build_invitation_link;
    use http::{HeaderMap, HeaderName, HeaderValue};

    #[test]
    fn invitation_link_uses_origin_when_present() {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::ORIGIN,
            HeaderValue::from_static("https://hub.niffler.org"),
        );

        assert_eq!(
            build_invitation_link(&headers, "ABC123"),
            "https://hub.niffler.org/register?invite=ABC123"
        );
    }

    #[test]
    fn invitation_link_uses_forwarded_host_without_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
        headers.insert(
            HeaderName::from_static("x-forwarded-host"),
            HeaderValue::from_static("hub.niffler.org"),
        );

        assert_eq!(
            build_invitation_link(&headers, "ABC123"),
            "https://hub.niffler.org/register?invite=ABC123"
        );
    }
}
