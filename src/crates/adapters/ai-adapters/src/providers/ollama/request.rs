//! Ollama request builder and stream sender
//!
//! Ollama's `/api/chat` endpoint is OpenAI-compatible for chat completions.
//! This module builds requests and handles streaming responses.

use crate::client::quirks::should_append_tool_stream;
use crate::client::sse::execute_sse_request;
use crate::client::{AIClient, StreamResponse};
use crate::providers::openai::common;
use crate::providers::shared;
use crate::stream::handle_openai_stream;
use crate::trace::ModelExchangeTraceConfig;
use crate::types::{Message, ModelRequestContext, ToolDefinition};
use anyhow::Result;
use log::debug;

fn try_build_request_body_with_context(
    client: &AIClient,
    url: &str,
    ollama_messages: Vec<serde_json::Value>,
    ollama_tools: Option<Vec<serde_json::Value>>,
    extra_body: Option<serde_json::Value>,
    request_context: Option<&ModelRequestContext>,
) -> Result<serde_json::Value> {
    let mut request_body = serde_json::json!({
        "model": client.config.model,
        "messages": ollama_messages,
        "stream": true
    });

    let model_name = client.config.model.to_lowercase();

    if should_append_tool_stream(url, &model_name) {
        request_body["tool_stream"] = serde_json::Value::Bool(true);
    }

    let base_reasoning_fields = shared::capture_reasoning_fields(
        &request_body,
        &["thinking", "enable_thinking", "reasoning_effort"],
        &[],
    );

    if let Some(max_tokens) = client.config.max_tokens {
        request_body["max_tokens"] = serde_json::json!(max_tokens);
    }

    let protected_keys = &[
        "model",
        "messages",
        "stream",
        "max_tokens",
        "tool_stream",
        "tools",
    ];
    if let Some(preset) = client.model_reasoning_preset.as_ref() {
        shared::apply_reasoning_actions(
            preset,
            &mut request_body,
            protected_keys,
            &[],
            |action, body| {
                compile_chat_reasoning_action(preset, action, body, url, &client.config.model)
            },
        )?;
    }

    let protected_body = shared::protect_request_body(
        client,
        &mut request_body,
        &["model", "messages", "stream", "max_tokens", "tool_stream"],
        &[],
    );

    if let Some(extra) = extra_body {
        if let Some(extra_obj) = extra.as_object() {
            shared::merge_extra_body(&mut request_body, extra_obj);
            shared::log_extra_body_keys("ai::ollama_stream_request", extra_obj);
        }
    }

    shared::restore_protected_body(&mut request_body, protected_body);
    if let Some(preset) = client.selected_reasoning_preset.as_ref() {
        shared::reset_reasoning_fields(
            &mut request_body,
            base_reasoning_fields.as_ref(),
            &["thinking", "enable_thinking", "reasoning_effort"],
            &[],
        );
        shared::apply_reasoning_actions(
            preset,
            &mut request_body,
            protected_keys,
            &[],
            |action, body| {
                compile_chat_reasoning_action(preset, action, body, url, &client.config.model)
            },
        )?;
    }

    if let Some(request_obj) = request_body.as_object_mut() {
        if let Some(existing_n) = request_obj.remove("n") {
            log::warn!(
                target: "ai::ollama_stream_request",
                "Removed custom request field n={} because the stream processor only handles the first choice",
                existing_n
            );
        }
    }
    if let Some(schema) = request_context.and_then(|context| context.output_schema.as_ref()) {
        request_body["response_format"] = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "alex_output",
                "strict": true,
                "schema": schema
            }
        });
    }

    shared::log_request_body(
        "ai::ollama_stream_request",
        "Ollama stream request body (excluding tools):",
        &request_body,
    );

    common::attach_tools(&mut request_body, ollama_tools, "ai::ollama_stream_request");

    Ok(request_body)
}

pub(crate) fn try_build_request_body(
    client: &AIClient,
    url: &str,
    ollama_messages: Vec<serde_json::Value>,
    ollama_tools: Option<Vec<serde_json::Value>>,
    extra_body: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    try_build_request_body_with_context(client, url, ollama_messages, ollama_tools, extra_body, None)
}

#[cfg(test)]
pub(crate) fn build_request_body(
    client: &AIClient,
    url: &str,
    ollama_messages: Vec<serde_json::Value>,
    ollama_tools: Option<Vec<serde_json::Value>>,
    extra_body: Option<serde_json::Value>,
) -> serde_json::Value {
    try_build_request_body(client, url, ollama_messages, ollama_tools, extra_body)
        .expect("request body should compile")
}

#[cfg(test)]
pub(crate) fn build_request_body_with_context(
    client: &AIClient,
    url: &str,
    ollama_messages: Vec<serde_json::Value>,
    ollama_tools: Option<Vec<serde_json::Value>>,
    extra_body: Option<serde_json::Value>,
    request_context: Option<&ModelRequestContext>,
) -> serde_json::Value {
    try_build_request_body_with_context(
        client,
        url,
        ollama_messages,
        ollama_tools,
        extra_body,
        request_context,
    )
    .expect("request body should compile")
}

fn compile_chat_reasoning_action(
    preset: &crate::types::ReasoningPresetDescriptor,
    action: &crate::types::ReasoningPresetAction,
    request_body: &mut serde_json::Value,
    _url: &str,
    configured_model: &str,
) -> Result<bool> {
    let _execution_provider = preset.execution_provider.as_deref().unwrap_or("ollama");
    let _execution_model = preset
        .execution_model
        .as_deref()
        .unwrap_or(configured_model)
        .trim()
        .to_ascii_lowercase();
    let is_generic_reasoning = shared::is_generic_reasoning_preset(preset);

    if is_generic_reasoning {
        return match action {
            crate::types::ReasoningPresetAction::Effort { value } => {
                let normalized = shared::normalize_generic_reasoning_effort(value).ok_or_else(|| {
                    anyhow::anyhow!("Generic reasoning effort '{}' is unsupported", value)
                })?;
                // Ollama doesn't have native reasoning support, use OpenAI-compatible fields
                request_body["reasoning_effort"] = serde_json::json!(normalized);
                Ok(true)
            }
            crate::types::ReasoningPresetAction::Toggle { enabled: true } => {
                request_body["reasoning_effort"] = serde_json::json!("medium");
                Ok(true)
            }
            crate::types::ReasoningPresetAction::Toggle { enabled: false } => {
                request_body.as_object_mut().map(|body| body.remove("reasoning_effort"));
                Ok(true)
            }
            crate::types::ReasoningPresetAction::BudgetTokens { .. } => Ok(false),
            crate::types::ReasoningPresetAction::RequestPatch { .. } => {
                unreachable!("patches are compiled by shared code")
            }
        };
    }

    // For non-generic presets, try to apply provider-specific reasoning
    // Ollama doesn't have native reasoning, so we only support generic
    Ok(false)
}

pub(crate) async fn send_stream(
    client: &AIClient,
    messages: Vec<Message>,
    tools: Option<Vec<ToolDefinition>>,
    extra_body: Option<serde_json::Value>,
    max_tries: usize,
    trace: Option<ModelExchangeTraceConfig>,
    request_context: Option<ModelRequestContext>,
) -> Result<StreamResponse> {
    let url = client.config.request_url.clone();
    let request_context = shared::prepare_request_context(client, request_context);
    debug!(
        "Ollama config: model={}, request_url={}, max_tries={}",
        client.config.model, client.config.request_url, max_tries
    );

    let ollama_messages = crate::providers::ollama::OllamaMessageConverter::convert_messages(messages);
    let ollama_tools = crate::providers::ollama::OllamaMessageConverter::convert_tools(tools);
    let request_body = try_build_request_body_with_context(
        client,
        &url,
        ollama_messages,
        ollama_tools,
        extra_body,
        request_context.as_ref(),
    )?;
    let inline_think_in_text = client.config.inline_think_in_text;
    let idle_timeout = client.stream_options.idle_timeout;
    let ttft_timeout = client.stream_options.ttft_timeout;

    execute_sse_request(
        "Ollama Streaming API",
        &url,
        &request_body,
        max_tries,
        ttft_timeout,
        trace,
        || {
            shared::apply_affinity_headers(
                client,
                apply_headers(client, client.client.post(&url), &url),
                &url,
                request_context.as_ref(),
            )
        },
        move |response, tx, tx_raw, remaining_ttft_timeout| {
            handle_openai_stream(
                response,
                tx,
                tx_raw,
                inline_think_in_text,
                remaining_ttft_timeout,
                idle_timeout,
            )
        },
    )
    .await
}

fn apply_headers(
    client: &AIClient,
    builder: reqwest::RequestBuilder,
    _url: &str,
) -> reqwest::RequestBuilder {
    shared::apply_header_policy(client, builder, |mut builder| {
        builder = builder.header("Content-Type", "application/json");
        // Ollama typically doesn't require authentication for local instances
        // but supports Bearer token if configured
        if !client.config.api_key.is_empty() && client.config.api_key != "ollama" {
            builder = builder.header("Authorization", format!("Bearer {}", client.config.api_key));
        }
        builder
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AIConfig;
    use serde_json::json;

    fn make_test_client() -> AIClient {
        AIClient::new(AIConfig {
            name: "ollama-test".to_string(),
            base_url: "http://localhost:11434".to_string(),
            request_url: "http://localhost:11434/api/chat".to_string(),
            api_key: "ollama".to_string(),
            model: "mistral".to_string(),
            format: "ollama".to_string(),
            context_window: 32768,
            max_tokens: Some(8192),
            temperature: None,
            top_p: None,
            inline_think_in_text: false,
            custom_headers: None,
            custom_headers_mode: None,
            skip_ssl_verify: false,
            custom_request_body: None,
            custom_request_body_mode: None,
        })
    }

    #[test]
    fn builds_basic_request_body() {
        let client = make_test_client();
        let body = build_request_body(
            &client,
            "http://localhost:11434/api/chat",
            vec![json!({"role": "user", "content": "Hello"})],
            None,
            None,
        );
        assert_eq!(body["model"], "mistral");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hello");
    }

    #[test]
    fn applies_max_tokens() {
        let client = make_test_client();
        let body = build_request_body(
            &client,
            "http://localhost:11434/api/chat",
            vec![],
            None,
            None,
        );
        assert_eq!(body["max_tokens"], 8192);
    }

    #[test]
    fn applies_reasoning_effort_for_generic_preset() {
        use crate::types::{ReasoningPresetAction, ReasoningPresetDescriptor};
        use crate::providers::shared::GENERIC_REASONING_PROVIDER_ID;
use alex_core_types::ReasoningPresetSource;
        
        let mut client = make_test_client();
        client.model_reasoning_preset = Some(ReasoningPresetDescriptor {
            id: "test".to_string(),
            label: "Test".to_string(),
            order: 0,
            actions: vec![ReasoningPresetAction::Effort { value: "high".to_string() }],
            source: ReasoningPresetSource::ModelsDev,
            execution_provider: Some(GENERIC_REASONING_PROVIDER_ID.to_string()),
            execution_model: Some("mistral".to_string()),
        });

        let body = build_request_body(
            &client,
            "http://localhost:11434/api/chat",
            vec![],
            None,
            None,
        );
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn omits_auth_when_api_key_is_ollama() {
        let client = make_test_client();
        let request = apply_headers(&client, client.client.post("http://localhost:11434/api/chat"), "http://localhost:11434/api/chat")
            .build()
            .unwrap();
        assert!(!request.headers().contains_key("authorization"));
    }

    #[test]
    fn includes_auth_when_custom_api_key() {
        let mut client = make_test_client();
        client.config.api_key = "my-custom-key".to_string();
        let request = apply_headers(&client, client.client.post("http://localhost:11434/api/chat"), "http://localhost:11434/api/chat")
            .build()
            .unwrap();
        assert_eq!(request.headers()["authorization"], "Bearer my-custom-key");
    }
}