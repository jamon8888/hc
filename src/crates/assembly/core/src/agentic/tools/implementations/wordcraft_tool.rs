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
const OPERATIONS: &[&str] = &[
    "list_commands",
    "execute",
    "batch",
    "get_text",
    "inspect_document",
    "render_page",
    "save_document",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
        let expected_revision = input
            .get("expectedRevision")
            .and_then(Value::as_u64)
            .unwrap_or(document.revision);
        let agent_session = context
            .session_id
            .as_deref()
            .filter(|session| !session.trim().is_empty())
            .unwrap_or(document.session_id.as_str());
        let result = get_global_wordcraft_bridge()
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

        Ok(vec![ToolResult::ok(
            json!({
                "operation": operation,
                "documentId": document.document_id,
                "sessionId": document.session_id,
                "result": result,
            }),
            None,
        )])
    }
}
