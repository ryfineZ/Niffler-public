use axum::{body::Body, http, response::Response};
use serde_json::json;

use super::{
    base_url_from_request, build_auth_json_response, resolve_authenticated_local_user, AppState,
    GatewayPublicRequestContext,
};

pub(super) async fn handle_users_me_public_base_url_get(
    state: &AppState,
    request_context: &GatewayPublicRequestContext,
    headers: &http::HeaderMap,
) -> Response<Body> {
    if let Err(response) = resolve_authenticated_local_user(state, request_context, headers).await {
        return response;
    }

    build_auth_json_response(
        http::StatusCode::OK,
        json!({
            "public_base_url": base_url_from_request(headers, request_context),
        }),
        None,
    )
}
