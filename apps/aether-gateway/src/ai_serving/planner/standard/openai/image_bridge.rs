use serde_json::{json, Map, Value};

use crate::ai_serving::CODEX_OPENAI_IMAGE_DEFAULT_MODEL;

const OPENAI_IMAGE_TOOL_OPTION_KEYS: &[&str] = &[
    "size",
    "quality",
    "background",
    "output_format",
    "output_compression",
    "moderation",
    "input_fidelity",
    "partial_images",
];

pub(super) fn openai_image_bridge_main_model(
    request_model: Option<&str>,
    requested_model: &str,
) -> Option<String> {
    non_image_model(request_model)
        .or_else(|| non_image_model(Some(requested_model)))
        .map(ToOwned::to_owned)
}

pub(super) fn build_openai_image_generation_tool(
    mut tool: Map<String, Value>,
    body_options: &Map<String, Value>,
    request_model: Option<&str>,
    requested_model: &str,
    mapped_image_model: Option<&str>,
) -> Value {
    tool.insert("type".to_string(), json!("image_generation"));
    if !tool.contains_key("output_format") {
        if let Some(format) = tool
            .get("format")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            tool.insert("output_format".to_string(), json!(format));
        }
    }
    tool.remove("format");

    for key in OPENAI_IMAGE_TOOL_OPTION_KEYS {
        if !tool.contains_key(*key) {
            if let Some(value) = body_options.get(*key) {
                tool.insert((*key).to_string(), value.clone());
            }
        }
    }

    if let Some(model) = non_empty_model(mapped_image_model) {
        tool.insert("model".to_string(), json!(model));
    } else {
        let model_is_empty = tool
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_none_or(str::is_empty);
        if model_is_empty {
            tool.insert(
                "model".to_string(),
                json!(openai_image_tool_model(
                    request_model,
                    requested_model,
                    None
                )),
            );
        }
    }

    Value::Object(tool)
}

pub(super) fn openai_image_generation_tool_choice() -> Value {
    json!({ "type": "image_generation" })
}

pub(super) fn collect_openai_image_body_options(object: &Map<String, Value>) -> Map<String, Value> {
    let mut options = Map::new();
    for key in OPENAI_IMAGE_TOOL_OPTION_KEYS {
        if let Some(value) = object.get(*key) {
            options.insert((*key).to_string(), value.clone());
        }
    }
    options
}

pub(super) fn openai_image_tool_model(
    request_model: Option<&str>,
    requested_model: &str,
    mapped_image_model: Option<&str>,
) -> String {
    non_empty_model(mapped_image_model)
        .or_else(|| image_model(request_model))
        .or_else(|| image_model(Some(requested_model)))
        .unwrap_or(CODEX_OPENAI_IMAGE_DEFAULT_MODEL)
        .to_string()
}

fn non_empty_model(model: Option<&str>) -> Option<&str> {
    model.map(str::trim).filter(|value| !value.is_empty())
}

fn image_model(model: Option<&str>) -> Option<&str> {
    non_empty_model(model).filter(|value| model_is_image_model(value))
}

fn non_image_model(model: Option<&str>) -> Option<&str> {
    non_empty_model(model).filter(|value| !model_is_image_model(value))
}

fn model_is_image_model(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    model.starts_with("gpt-image-") || model.starts_with("dall-e-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_main_model_and_image_tool_model() {
        assert_eq!(
            openai_image_bridge_main_model(Some("gpt-5.5"), "gpt-image-2"),
            Some("gpt-5.5".to_string())
        );
        assert_eq!(
            openai_image_bridge_main_model(Some("gpt-image-2"), "gpt-image-2"),
            None
        );

        let tool = build_openai_image_generation_tool(
            Map::new(),
            &Map::new(),
            Some("gpt-image-2"),
            "gpt-image-2",
            None,
        );

        assert_eq!(tool["type"], "image_generation");
        assert_eq!(tool["model"], "gpt-image-2");
    }

    #[test]
    fn image_tool_uses_mapped_image_model_even_when_name_is_not_openai_style() {
        let tool = build_openai_image_generation_tool(
            Map::new(),
            &Map::new(),
            Some("gpt-image-2"),
            "gpt-image-2",
            Some("grok-imagine-image-pro"),
        );

        assert_eq!(tool["type"], "image_generation");
        assert_eq!(tool["model"], "grok-imagine-image-pro");
    }

    #[test]
    fn mapped_image_model_overrides_existing_tool_model() {
        let mut existing = Map::new();
        existing.insert("model".to_string(), json!("gpt-image-2"));
        let tool = build_openai_image_generation_tool(
            existing,
            &Map::new(),
            Some("gpt-image-2"),
            "gpt-image-2",
            Some("upstream-image-model"),
        );

        assert_eq!(tool["model"], "upstream-image-model");
    }
}
