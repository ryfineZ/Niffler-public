use super::super::errors::build_internal_control_error_response;
use crate::handlers::admin::request::AdminProviderOAuthTemplate;
use aether_oauth::provider::{ProviderOAuthService, ProviderOAuthTransportContext};
use axum::{body::Body, http, response::Response};
use serde_json::json;
use url::form_urlencoded;

pub(crate) fn build_provider_oauth_start_response(
    template: AdminProviderOAuthTemplate,
    ctx: ProviderOAuthTransportContext,
    nonce: &str,
    code_challenge: Option<&str>,
) -> Result<serde_json::Value, Response<Body>> {
    let authorization_url =
        match build_provider_oauth_authorization_url(template, &ctx, nonce, code_challenge) {
            Ok(authorization_url) => authorization_url,
            Err(_) if !template.client_id.trim().is_empty() => {
                build_provider_oauth_authorization_url_legacy(template, nonce, code_challenge)
            }
            Err(response) => return Err(response),
        };

    Ok(json!({
        "authorization_url": authorization_url,
        "redirect_uri": template.redirect_uri,
        "provider_type": template.provider_type,
        "instructions": "1) 打开 authorization_url 完成授权\n2) 授权后会跳转到 redirect_uri（localhost）\n3) 复制浏览器地址栏完整 URL，调用 complete 接口粘贴 callback_url",
    }))
}

fn build_provider_oauth_authorization_url(
    template: AdminProviderOAuthTemplate,
    ctx: &ProviderOAuthTransportContext,
    nonce: &str,
    code_challenge: Option<&str>,
) -> Result<String, Response<Body>> {
    ProviderOAuthService::with_builtin_adapters()
        .build_authorize_url(ctx, nonce, code_challenge)
        .map(|response| response.authorize_url)
        .map_err(|error| {
            build_internal_control_error_response(
                http::StatusCode::BAD_REQUEST,
                format!("OAuth 授权配置无效: {error}"),
            )
        })
}

fn build_provider_oauth_authorization_url_legacy(
    template: AdminProviderOAuthTemplate,
    nonce: &str,
    code_challenge: Option<&str>,
) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("client_id", template.client_id);
    serializer.append_pair("response_type", "code");
    serializer.append_pair("redirect_uri", template.redirect_uri);
    serializer.append_pair("scope", &template.scopes.join(" "));
    serializer.append_pair("state", nonce);
    if template.provider_type == "codex" {
        serializer.append_pair("prompt", "login");
        serializer.append_pair("id_token_add_organizations", "true");
        serializer.append_pair("codex_cli_simplified_flow", "true");
    }
    if template.use_pkce {
        if let Some(code_challenge) = code_challenge {
            serializer.append_pair("code_challenge", code_challenge);
            serializer.append_pair("code_challenge_method", "S256");
        }
    }

    format!("{}?{}", template.authorize_url, serializer.finish())
}

#[cfg(test)]
mod tests {
    use super::build_provider_oauth_start_response;
    use aether_oauth::provider::ProviderOAuthTransportContext;
    use aether_provider_transport::provider_types::provider_type_admin_oauth_template;
    use serde_json::json;

    fn test_context(
        provider_type: &str,
        provider_config: Option<serde_json::Value>,
    ) -> ProviderOAuthTransportContext {
        ProviderOAuthTransportContext {
            provider_id: "provider-1".to_string(),
            provider_type: provider_type.to_string(),
            endpoint_id: None,
            key_id: None,
            auth_type: Some("oauth".to_string()),
            decrypted_api_key: None,
            decrypted_auth_config: None,
            provider_config,
            endpoint_config: None,
            key_config: None,
            network: aether_oauth::network::OAuthNetworkContext::provider_operation(None),
        }
    }

    #[test]
    fn gemini_cli_start_requires_configured_oauth_client() {
        let template =
            provider_type_admin_oauth_template("gemini_cli").expect("template should exist");

        let response = build_provider_oauth_start_response(
            template,
            test_context("gemini_cli", None),
            "nonce",
            None,
        );

        assert!(response.is_err());
    }

    #[test]
    fn gemini_cli_start_uses_provider_configured_oauth_client() {
        let template =
            provider_type_admin_oauth_template("gemini_cli").expect("template should exist");
        let response = build_provider_oauth_start_response(
            template,
            test_context(
                "gemini_cli",
                Some(json!({
                    "oauth_client": {
                        "client_id": "test-gemini-cli-client-id",
                        "client_secret": "test-gemini-cli-client-secret"
                    }
                })),
            ),
            "nonce",
            None,
        )
        .expect("start response should build");

        assert!(response["authorization_url"]
            .as_str()
            .expect("authorization url should be a string")
            .contains("client_id=test-gemini-cli-client-id"));
    }
}
