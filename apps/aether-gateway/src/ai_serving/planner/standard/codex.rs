#[cfg(test)]
#[path = "codex/tests.rs"]
mod tests;

pub(crate) use crate::ai_serving::{
    apply_codex_openai_responses_special_body_edits,
    apply_codex_openai_responses_special_body_edits_with_bridge_config,
    apply_codex_openai_responses_special_body_edits_with_bridge_model,
    apply_codex_openai_responses_special_headers,
};

const OPENAI_RESPONSES_IMAGE_GENERATION_TOOL_ENABLED_CONFIG_KEY: &str =
    "openai_responses_image_generation_tool_enabled";

pub(crate) fn codex_openai_image_bridge_model_from_provider_config(
    provider_config: Option<&serde_json::Value>,
) -> Option<&str> {
    provider_config
        .and_then(serde_json::Value::as_object)
        .and_then(|config| config.get("codex_image_generation_base_model"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn openai_responses_image_generation_tool_enabled_from_transport_config(
    provider_type: &str,
    provider_config: Option<&serde_json::Value>,
    endpoint_config: Option<&serde_json::Value>,
) -> bool {
    if let Some(enabled) =
        openai_responses_image_generation_tool_enabled_config_value(endpoint_config).or_else(|| {
            openai_responses_image_generation_tool_enabled_config_value(provider_config)
        })
    {
        return enabled;
    }

    matches!(
        provider_type.trim().to_ascii_lowercase().as_str(),
        "codex" | "chatgpt_web"
    )
}

fn openai_responses_image_generation_tool_enabled_config_value(
    config: Option<&serde_json::Value>,
) -> Option<bool> {
    config
        .and_then(serde_json::Value::as_object)
        .and_then(|config| config.get(OPENAI_RESPONSES_IMAGE_GENERATION_TOOL_ENABLED_CONFIG_KEY))
        .and_then(serde_json::Value::as_bool)
}
