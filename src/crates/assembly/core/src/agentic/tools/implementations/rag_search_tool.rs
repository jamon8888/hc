//! `rag_search` tool — PII-safe search over the indexed workspace.
//!
//! The tool wrapper is the only part of the RAG surface `openbitfun-core`
//! owns. It never sees the basemind client: it takes the real `session_id`
//! from the tool context, asks the injected `RagSearchPort` for the hits,
//! and returns them. The concrete pipeline — query resolution through the
//! session vault, the basemind search, and the re-tokenization of every hit
//! value into an opaque session token — lives in
//! `openbitfun-basemind-integration` (D11, SPEC §4).
//!
//! The hits that reach the model carry session tokens, never the underlying
//! values, so the PII guarantee holds at the LLM boundary (I1, D3).

use crate::agentic::tools::framework::{Tool, ToolRenderOptions, ToolResult, ToolUseContext};
use crate::util::errors::OpenBitFunResult;
use async_trait::async_trait;
use openbitfun_runtime_ports::{RagSearchError, RagSearchErrorKind, RagSearchRequest};
use serde_json::{json, Value};

/// Default number of hits the tool asks for.
const DEFAULT_TOP_K: u32 = 8;
/// The largest number of hits the tool asks for.
const MAX_TOP_K: u32 = 50;

/// `rag_search` tool — searches the indexed workspace without leaking PII.
pub struct RagSearchTool;

impl Default for RagSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RagSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RagSearchTool {
    fn name(&self) -> &str {
        "rag_search"
    }

    async fn description(&self) -> OpenBitFunResult<String> {
        Ok(r#"Search the indexed workspace and answer from what it finds.

Use this tool to look up facts about the current repository — how a subsystem works, where a symbol lives, what a file contains — before reading files one by one. It searches the workspace index, so it answers from a map of the code rather than from raw file bodies.

The query is plain language. A name, a path fragment, or a question all work.

Every personal value in the results is replaced by an opaque token, so a result can quote a person or an account without carrying the value itself."#
            .to_string())
    }

    fn short_description(&self) -> String {
        "Search the indexed workspace without exposing personal data.".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to look for: a name, a path fragment, or a question."
                },
                "top_k": {
                    "type": "integer",
                    "description": "How many hits to return.",
                    "minimum": 1,
                    "maximum": 50
                }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: Option<&Value>) -> bool {
        true
    }

    fn render_tool_use_message(&self, input: &Value, _options: &ToolRenderOptions) -> String {
        input
            .get("query")
            .and_then(Value::as_str)
            .map(|query| format!("Search the workspace: {query}"))
            .unwrap_or_else(|| "Search the workspace".to_string())
    }

    fn render_tool_result_message(&self, output: &Value) -> String {
        let hits = output.get("hits").and_then(Value::as_array).map_or(0, Vec::len);
        format!("Search returned {hits} hits")
    }

    async fn call_impl(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> OpenBitFunResult<Vec<ToolResult>> {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .ok_or_else(|| {
                crate::util::errors::OpenBitFunError::validation("query must not be empty")
            })?;
        let top_k = input
            .get("top_k")
            .and_then(Value::as_u64)
            .map(|value| value.clamp(1, MAX_TOP_K as u64) as u32)
            .unwrap_or(DEFAULT_TOP_K);

        // The real session id, never a placeholder: the port resolves the
        // session's vault payload with it, and a wrong id would resolve
        // against the wrong salt.
        let session_id = context
            .session_id
            .clone()
            .ok_or_else(|| {
                crate::util::errors::OpenBitFunError::validation(
                    "rag_search needs a session to scope the rehydration map",
                )
            })?;
        let workspace_root = context
            .workspace_root()
            .map(|root| root.to_string_lossy().to_string())
            .ok_or_else(|| {
                crate::util::errors::OpenBitFunError::validation(
                    "rag_search needs an open workspace to search",
                )
            })?;

        let port = context
            .runtime_handles
            .rag_search_port()
            .cloned()
            .ok_or_else(|| {
                crate::util::errors::OpenBitFunError::service(
                    "rag_search is not available in this runtime: no search backend is configured",
                )
            })?;

        let response = port
            .search(RagSearchRequest {
                query: query.to_string(),
                top_k,
                session_id,
                workspace_root,
            })
            .await
            .map_err(|error| rag_search_error(error))?;

        let hits = serde_json::to_value(&response.hits)
            .unwrap_or_else(|_| Value::Array(Vec::new()));
        Ok(vec![ToolResult::ok(
            json!({ "hits": hits }),
            Some(format!("Search returned {} hits", response.hits.len())),
        )])
    }
}

/// Turn a port error into the tool's error type.
///
/// A missing session payload is a hard failure rather than an empty result:
/// the port refuses to answer with raw text when it cannot resolve the
/// session's tokens (D7), and the tool must not paper over that.
fn rag_search_error(error: RagSearchError) -> crate::util::errors::OpenBitFunError {
    use crate::util::errors::OpenBitFunError;
    match error.kind {
        RagSearchErrorKind::InvalidRequest => {
            OpenBitFunError::validation(format!("rag_search: {}", error.message))
        }
        RagSearchErrorKind::SessionUnavailable => OpenBitFunError::service(format!(
            "rag_search could not resolve the session: {}",
            error.message
        )),
        RagSearchErrorKind::Transport => {
            OpenBitFunError::service(format!("rag_search could not reach the index: {}", error.message))
        }
        RagSearchErrorKind::InvalidResponse => OpenBitFunError::service(format!(
            "rag_search got an unusable answer from the index: {}",
            error.message
        )),
        RagSearchErrorKind::Failed => {
            OpenBitFunError::service(format!("rag_search failed: {}", error.message))
        }
    }
}

#[cfg(all(test, feature = "tools-rag"))]
mod tests {
    use super::*;
    use crate::agentic::tools::framework::{Tool, ToolUseContext};
    use async_trait::async_trait;
    use openbitfun_runtime_ports::{
        RagSearchError, RagSearchErrorKind, RagSearchHit, RagSearchPort, RagSearchResponse,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// A port that records what it was asked and answers with a fixed reply.
    struct StubPort {
        seen: Mutex<Option<RagSearchRequest>>,
        response: Result<RagSearchResponse, RagSearchError>,
    }

    #[async_trait]
    impl RagSearchPort for StubPort {
        async fn search(
            &self,
            request: RagSearchRequest,
        ) -> Result<RagSearchResponse, RagSearchError> {
            *self.seen.lock().unwrap() = Some(request);
            self.response.clone()
        }
    }

    fn test_context(port: Option<Arc<dyn RagSearchPort>>) -> ToolUseContext {
        let mut handles = openbitfun_runtime_ports::ToolRuntimeHandles::default();
        handles = handles.with_rag_search_port(port);
        ToolUseContext {
            tool_call_id: None,
            agent_type: None,
            session_id: Some("session-1".to_string()),
            dialog_turn_id: None,
            workspace: None,
            loaded_deferred_tool_specs: Vec::new(),
            primary_model_facts: tool_runtime::context::PrimaryModelFacts::default(),
            custom_data: HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: Default::default(),
            runtime_handles: handles,
        }
    }

    fn ok_response() -> RagSearchResponse {
        RagSearchResponse {
            hits: vec![RagSearchHit {
                chunk: "a chunk".to_string(),
                source_doc: "src/lib.rs".to_string(),
                score: 0.5,
                entities: Vec::new(),
            }],
        }
    }

    #[tokio::test]
    async fn the_tool_delegates_to_the_injected_port() {
        let port = Arc::new(StubPort {
            seen: Mutex::new(None),
            response: Ok(ok_response()),
        });
        let seen = port.clone();
        let context = test_context(Some(port));

        let results = RagSearchTool::new()
            .call_impl(&serde_json::json!({ "query": "how does login work" }), &context)
            .await
            .expect("the search should succeed");

        assert_eq!(results.len(), 1);
        let request = seen.seen.lock().unwrap().clone().expect("the port was called");
        assert_eq!(request.query, "how does login work");
        assert_eq!(request.session_id, "session-1");
        // The tool never invents a session: the real one travels with the
        // request, because the port resolves the vault with it.
        assert_eq!(request.top_k, DEFAULT_TOP_K);
    }

    #[tokio::test]
    async fn an_empty_query_is_refused_before_the_port_is_called() {
        let port = Arc::new(StubPort {
            seen: Mutex::new(None),
            response: Ok(ok_response()),
        });
        let seen = port.clone();
        let context = test_context(Some(port));

        let error = RagSearchTool::new()
            .call_impl(&serde_json::json!({ "query": "   " }), &context)
            .await
            .expect_err("an empty query must be refused");
        assert!(seen.seen.lock().unwrap().is_none());
        assert!(error.to_string().contains("query"));
    }

    #[tokio::test]
    async fn a_missing_port_is_an_explicit_error_not_an_empty_result() {
        let context = test_context(None);
        let error = RagSearchTool::new()
            .call_impl(&serde_json::json!({ "query": "anything" }), &context)
            .await
            .expect_err("a runtime without a search backend must fail loudly");
        assert!(error.to_string().contains("rag_search"));
    }

    #[tokio::test]
    async fn a_missing_session_vault_is_reported_as_a_hard_failure() {
        let port = Arc::new(StubPort {
            seen: Mutex::new(None),
            response: Err(RagSearchError::new(
                RagSearchErrorKind::SessionUnavailable,
                "session payload unavailable",
            )),
        });
        let context = test_context(Some(port));
        let error = RagSearchTool::new()
            .call_impl(&serde_json::json!({ "query": "anything" }), &context)
            .await
            .expect_err("an unresolvable session must not answer with raw text");
        assert!(error.to_string().contains("session"));
    }

    #[tokio::test]
    async fn the_top_k_is_clamped_to_the_documented_range() {
        let port = Arc::new(StubPort {
            seen: Mutex::new(None),
            response: Ok(ok_response()),
        });
        let seen = port.clone();
        let context = test_context(Some(port));

        RagSearchTool::new()
            .call_impl(
                &serde_json::json!({ "query": "anything", "top_k": 9_999 }),
                &context,
            )
            .await
            .expect("the search should succeed");
        assert_eq!(seen.seen.lock().unwrap().clone().unwrap().top_k, MAX_TOP_K);
    }
}
