//! Agent control for the currently attached embedded WordCraft document.

use crate::agentic::tools::framework::{
    PermissionIntent, Tool, ToolRenderOptions, ToolResult, ToolUseContext, ValidationResult,
};
use crate::service::wordcraft_bridge::{
    get_global_wordcraft_bridge, WordcraftEmbeddedCommandRequest,
};
use crate::util::errors::{AlexError, AlexResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const DOCUMENT_CONTEXT_KEY: &str = "wordcraft_document_context";
const TRACK_CHANGES_COMMAND: &str = "review.trackChanges";
const OPERATIONS: &[&str] = &[
    "list_commands",
    "execute",
    "batch",
    "get_text",
    "inspect_document",
    "render_page",
    "save_document",
];

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DocumentAgentContext {
    kind: String,
    surface: String,
    document_id: String,
    session_id: String,
    surface_id: String,
    workspace_id: String,
    file_path: String,
    revision: u64,
    status: String,
    capabilities: Vec<String>,
}

pub struct WordcraftTool;

impl Default for WordcraftTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WordcraftTool {
    pub fn new() -> Self {
        Self
    }

    fn context<'a>(&self, context: &'a ToolUseContext) -> AlexResult<DocumentAgentContext> {
        let value = context
            .custom_data
            .get(DOCUMENT_CONTEXT_KEY)
            .ok_or_else(|| tool_error("No embedded WordCraft document is attached to this turn"))?;
        let value = match value {
            Value::String(raw) => serde_json::from_str(raw).map_err(|error| {
                tool_error(format!("Embedded WordCraft context is malformed: {error}"))
            })?,
            value => value.clone(),
        };
        serde_json::from_value(value).map_err(|error| {
            tool_error(format!("Embedded WordCraft context is malformed: {error}"))
        })
    }

    fn operation(input: &Value) -> AlexResult<&str> {
        input
            .get("operation")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|operation| OPERATIONS.contains(operation))
            .ok_or_else(|| {
                tool_error(format!(
                    "WordCraft operation must be one of: {}",
                    OPERATIONS.join(", ")
                ))
            })
    }
}

fn tool_error(message: impl Into<String>) -> AlexError {
    AlexError::tool(message.into())
}

fn read_operation(input: &Value) -> Option<&str> {
    input
        .get("operation")
        .and_then(Value::as_str)
        .map(str::trim)
}

fn track_changes_command() -> Value {
    json!({
        "command": TRACK_CHANGES_COMMAND,
        "params": { "value": true },
    })
}

fn command_name(command: &Value) -> Option<&str> {
    command.get("command").and_then(Value::as_str)
}

/// Ensure every agent batch enables tracked changes explicitly before any edit.
/// A false or malformed tracking command is rejected rather than silently
/// changing the user's review policy.
fn prepare_batch_params(params: &Value) -> AlexResult<Value> {
    let mut object = params
        .as_object()
        .cloned()
        .ok_or_else(|| tool_error("WordCraft batch params must be an object"))?;
    let commands = object
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| tool_error("WordCraft batch params must include a commands array"))?;

    let mut prepared = Vec::with_capacity(commands.len() + 1);
    for command in commands {
        if command_name(command) != Some(TRACK_CHANGES_COMMAND) {
            prepared.push(command.clone());
            continue;
        }
        let enabled = command
            .get("params")
            .and_then(|params| params.get("value"))
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                tool_error("WordCraft review.trackChanges must set params.value to true")
            })?;
        if !enabled {
            return Err(tool_error(
                "WordCraft agent edits cannot disable tracked changes",
            ));
        }
    }
    prepared.insert(0, track_changes_command());
    object.insert("commands".to_string(), Value::Array(prepared));
    Ok(Value::Object(object))
}

fn execute_command_name(params: &Value) -> AlexResult<&str> {
    params
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| tool_error("WordCraft execute params must include a command"))
}

fn validate_tracking_execute(params: &Value) -> AlexResult<()> {
    let enabled = params
        .get("params")
        .and_then(|params| params.get("value"))
        .and_then(Value::as_bool)
        .ok_or_else(|| tool_error("WordCraft review.trackChanges must set params.value to true"))?;
    if !enabled {
        return Err(tool_error(
            "WordCraft agent edits cannot disable tracked changes",
        ));
    }
    Ok(())
}

#[async_trait]
impl Tool for WordcraftTool {
    fn name(&self) -> &str {
        "Wordcraft"
    }

    async fn description(&self) -> AlexResult<String> {
        Ok("Act on the document currently visible in Alex's embedded WordCraft surface. Use get_text or inspect_document before editing, execute or batch for UI-aware changes, and save_document when the visible document is ready to persist.".to_string())
    }

    fn short_description(&self) -> String {
        "Read and edit the active document in the visible embedded WordCraft surface.".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": OPERATIONS,
                    "description": "The UI-aware WordCraft operation to run against the active document."
                },
                "params": {
                    "type": "object",
                    "description": "Operation-specific parameters. Use an empty object for list_commands, get_text, inspect_document, or save_document."
                },
                "expectedRevision": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional document revision guard. Defaults to the revision attached to this turn."
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    async fn is_available_in_context(&self, context: Option<&ToolUseContext>) -> bool {
        context
            .and_then(|context| self.context(context).ok())
            .is_some_and(|document| {
                document.kind == "document"
                    && document.surface == "wordcraft-embedded"
                    && document.status == "ready"
                    && document
                        .capabilities
                        .iter()
                        .any(|capability| OPERATIONS.contains(&capability.as_str()))
            })
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        false
    }

    fn permission_intents(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> AlexResult<Vec<PermissionIntent>> {
        let operation = read_operation(input).unwrap_or("unknown");
        if matches!(
            operation,
            "get_text" | "inspect_document" | "list_commands" | "render_page"
        ) {
            return Ok(Vec::new());
        }
        let document = self.context(context)?;
        Ok(vec![PermissionIntent::new(
            "wordcraft_document",
            vec![format!("{}:{operation}", document.file_path)],
        )])
    }

    async fn validate_input(
        &self,
        input: &Value,
        context: Option<&ToolUseContext>,
    ) -> ValidationResult {
        if let Some(context) = context {
            if let Err(error) = self.context(context) {
                return ValidationResult {
                    result: false,
                    message: Some(error.to_string()),
                    error_code: Some(409),
                    meta: None,
                };
            }
        }
        if let Err(error) = Self::operation(input) {
            return ValidationResult {
                result: false,
                message: Some(error.to_string()),
                error_code: Some(400),
                meta: None,
            };
        }
        if input
            .get("params")
            .is_some_and(|params| !params.is_object())
        {
            return ValidationResult {
                result: false,
                message: Some("WordCraft params must be an object".to_string()),
                error_code: Some(400),
                meta: None,
            };
        }
        ValidationResult::default()
    }

    fn render_tool_use_message(&self, input: &Value, options: &ToolRenderOptions) -> String {
        let operation = read_operation(input).unwrap_or("operation");
        if options.verbose {
            format!("Running WordCraft operation: {operation}")
        } else {
            format!("WordCraft: {operation}")
        }
    }

    async fn call_impl(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> AlexResult<Vec<ToolResult>> {
        let document = self.context(context)?;
        if document.kind != "document" || document.surface != "wordcraft-embedded" {
            return Err(tool_error(
                "The attached document is not an embedded WordCraft surface",
            ));
        }
        if document.status != "ready" {
            return Err(tool_error(format!(
                "The embedded WordCraft surface is not ready (status: {})",
                document.status
            )));
        }
        if context.is_remote() {
            return Err(tool_error(
                "Embedded WordCraft control is unavailable for remote workspaces",
            ));
        }
        if let Some(workspace_id) = context.workspace_id() {
            if workspace_id != document.workspace_id {
                return Err(tool_error(
                    "The attached WordCraft document belongs to a different workspace",
                ));
            }
        }

        let operation = Self::operation(input)?;
        if !document
            .capabilities
            .iter()
            .any(|capability| capability == operation)
        {
            return Err(tool_error(format!(
                "The visible WordCraft surface does not advertise '{operation}'"
            )));
        }
        let params = input.get("params").cloned().unwrap_or_else(|| json!({}));
        let params = match operation {
            "batch" => prepare_batch_params(&params)?,
            "execute" => {
                let command = execute_command_name(&params)?;
                if command == TRACK_CHANGES_COMMAND {
                    validate_tracking_execute(&params)?;
                }
                params
            }
            _ => params,
        };
        let mut expected_revision = input
            .get("expectedRevision")
            .and_then(Value::as_u64)
            .unwrap_or(document.revision);
        let agent_session = context
            .session_id
            .as_deref()
            .filter(|session| !session.trim().is_empty())
            .unwrap_or(document.session_id.as_str());
        let bridge = get_global_wordcraft_bridge();
        if operation == "execute" && execute_command_name(&params)? != TRACK_CHANGES_COMMAND {
            let tracking_result = bridge
                .request(WordcraftEmbeddedCommandRequest {
                    request_id: Uuid::new_v4().to_string(),
                    document_id: document.document_id.clone(),
                    session_id: document.session_id.clone(),
                    surface_id: document.surface_id.clone(),
                    workspace_id: document.workspace_id.clone(),
                    file_path: document.file_path.clone(),
                    expected_revision,
                    operation: operation.to_string(),
                    params: track_changes_command(),
                    author: format!("Alex Agent ({agent_session})"),
                })
                .await
                .map_err(tool_error)?;
            expected_revision = tracking_result
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or(expected_revision);
        }
        let result = bridge
            .request(WordcraftEmbeddedCommandRequest {
                request_id: Uuid::new_v4().to_string(),
                document_id: document.document_id.clone(),
                session_id: document.session_id.clone(),
                surface_id: document.surface_id.clone(),
                workspace_id: document.workspace_id.clone(),
                file_path: document.file_path.clone(),
                expected_revision,
                operation: operation.to_string(),
                params,
                author: format!("Alex Agent ({agent_session})"),
            })
            .await
            .map_err(tool_error)?;

        let save_result = if matches!(operation, "execute" | "batch") {
            let save_revision = result
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or(expected_revision);
            Some(
                match bridge
                    .request(WordcraftEmbeddedCommandRequest {
                        request_id: Uuid::new_v4().to_string(),
                        document_id: document.document_id.clone(),
                        session_id: document.session_id.clone(),
                        surface_id: document.surface_id.clone(),
                        workspace_id: document.workspace_id.clone(),
                        file_path: document.file_path.clone(),
                        expected_revision: save_revision,
                        operation: "save_document".to_string(),
                        params: json!({}),
                        author: format!("Alex Agent ({agent_session})"),
                    })
                    .await
                {
                    Ok(value) => json!({ "ok": true, "result": value }),
                    Err(error) => json!({ "ok": false, "error": error.to_string() }),
                },
            )
        } else {
            None
        };

        Ok(vec![ToolResult::ok(
            json!({
                "operation": operation,
                "documentId": document.document_id,
                "sessionId": document.session_id,
                "result": result,
                "durableSave": save_result,
            }),
            None,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::{prepare_batch_params, validate_tracking_execute, DocumentAgentContext};
    use serde_json::json;

    #[test]
    fn batch_starts_with_explicit_tracking_and_removes_duplicate_tracking_commands() {
        let params = prepare_batch_params(&json!({
            "keepGoing": true,
            "commands": [
                { "command": "type_text", "params": { "text": "Hello" } },
                { "command": "review.trackChanges", "params": { "value": true } },
            ],
        }))
        .expect("valid batch should be prepared");

        assert_eq!(
            params,
            json!({
                "keepGoing": true,
                "commands": [
                    { "command": "review.trackChanges", "params": { "value": true } },
                    { "command": "type_text", "params": { "text": "Hello" } },
                ],
            })
        );
    }

    #[test]
    fn batch_rejects_an_explicit_tracking_disable() {
        let error = prepare_batch_params(&json!({
            "commands": [
                { "command": "review.trackChanges", "params": { "value": false } },
            ],
        }))
        .expect_err("agent batches must not disable tracking");
        assert!(error.to_string().contains("cannot disable tracked changes"));
    }

    #[test]
    fn execute_tracking_requires_true_value() {
        let error = validate_tracking_execute(&json!({
            "command": "review.trackChanges",
            "params": { "value": false },
        }))
        .expect_err("tracking must be explicitly enabled");
        assert!(error.to_string().contains("cannot disable tracked changes"));
    }
    #[test]
    fn legacy_partial_context_defaults_to_an_explicitly_unavailable_context() {
        let context: DocumentAgentContext = serde_json::from_value(json!({
            "kind": "document",
            "surface": "wordcraft-embedded",
            "status": "ready",
        }))
        .expect("partial legacy context should remain readable");

        assert!(context.document_id.is_empty());
        assert!(context.session_id.is_empty());
        assert_eq!(context.revision, 0);
        assert!(context.capabilities.is_empty());
    }
}
