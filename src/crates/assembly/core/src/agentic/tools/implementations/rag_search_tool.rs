//! RAG search tool using Basemind integration

use crate::agentic::tools::framework::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[cfg(feature = "basemind")]
use openbitfun_basemind_integration::BasemindClient;

/// Tool for performing RAG search on the workspace using Basemind
#[cfg(feature = "basemind")]
pub struct RagSearchTool {
    basemind: Arc<BasemindClient>,
}

#[cfg(feature = "basemind")]
impl RagSearchTool {
    pub fn new(basemind: Arc<BasemindClient>) -> Self {
        Self { basemind }
    }
}

#[cfg(feature = "basemind")]
#[async_trait]
impl crate::agentic::tools::framework::Tool for RagSearchTool {
    fn name(&self) -> &str {
        "rag_search"
    }

    async fn description(&self) -> crate::util::errors::OpenBitFunResult<String> {
        Ok(r#"Performs a semantic search (RAG) over the workspace documents using Basemind.
Returns relevant document chunks with their sources and relevance scores.
The search is performed on redacted content only, ensuring no PII is exposed."#.to_string())
    }

    fn short_description(&self) -> String {
        "Semantic search over workspace documents (RAG)".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language query to search for"
                },
                "top_k": {
                    "type": "number",
                    "description": "Maximum number of results to return (default: 8)",
                    "minimum": 1,
                    "maximum": 50
                }
            },
            "required": ["query"]
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(&self, input: serde_json::Value, _context: &crate::agentic::tools::framework::ToolUseContext) -> crate::agentic::tools::framework::ToolResult {
        let query = match input["query"].as_str() {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return crate::agentic::tools::framework::ToolResult::error("query parameter is required and cannot be empty"),
        };

        let top_k = input["top_k"].as_u64().unwrap_or(8) as usize;

        let hits = match self.basemind.query_rag(query, top_k).await {
            Ok(h) => h,
            Err(e) => return crate::agentic::tools::framework::ToolResult::error(format!("RAG search failed: {}", e)),
        };

        crate::agentic::tools::framework::ToolResult::json(hits)
    }
}

#[cfg(feature = "basemind")]
pub fn register_rag_search_tool(
    registry: &mut crate::agentic::tools::registry::ToolRegistry,
    basemind: Arc<openbitfun_basemind_integration::BasemindClient>,
) {
    registry.register_tool(Arc::new(RagSearchTool::new(basemind)));
}