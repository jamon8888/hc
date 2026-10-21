//! PII-safe RAG search pipeline — RAG search on the open workspace with
//! session-scoped re-tokenization at the LLM boundary.
//!
//! Queries basemind's `memory mode="documents"` MCP tool against the **raw**
//! index (D6 amended): echoed session tokens are reversed through the session
//! vault before the search, and every hit value is minted into an opaque
//! session token before it can reach the LLM (D3/I1).
//!
//! This crate owns only the portable search pipeline; the `Tool` trait lives in
//! `openbitfun-core` (`tools-rag`), whose `rag_search_tool.rs` wraps
//! [`RagSearchTool`] through the `RagSearchPort` seam (D11).

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "basemind")]
use crate::identity_registry::IdentityRegistry;
#[cfg(feature = "basemind")]
use crate::ipc::BasemindClient;
#[cfg(feature = "basemind")]
use crate::query_resolution::QueryResolver;
#[cfg(feature = "basemind")]
use crate::session_vault::SessionVault;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSearchInput {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

#[cfg(feature = "basemind")]
fn default_top_k() -> usize {
    8
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSearchOutput {
    pub hits: Vec<crate::query_resolution::RetokenizedHit>,
}

#[cfg(feature = "basemind")]
pub struct RagSearchTool {
    client: BasemindClient,
    resolver: QueryResolver,
}

#[cfg(feature = "basemind")]
impl RagSearchTool {
    pub fn new(client: BasemindClient, registry: IdentityRegistry, vault: SessionVault) -> Self {
        let resolver = QueryResolver::new(client.clone(), registry, vault);
        Self { client, resolver }
    }

    /// Run a PII-safe workspace search.
    ///
    /// 1. Resolve echoed session tokens to raw values through the session
    ///    vault (the index is raw, D6 amended).
    /// 2. Search basemind documents with the resolved query.
    /// 3. Re-tokenize every hit value into a session token at the LLM
    ///    boundary (D3), unioning a `redact_text` safety pass over each chunk.
    ///
    /// `session_id` selects the session payload; the caller passes the real
    /// session id from the tool context. Fails hard when the session payload
    /// cannot be loaded (D7: never return raw entity text).
    pub async fn search(
        &mut self,
        query: &str,
        top_k: usize,
        session_id: &str,
        workspace_root: &str,
    ) -> Result<RagSearchOutput> {
        // 1. Resolve echoed session tokens through the vault (D7 on miss)
        let rewritten_query = self.resolver.rewrite_query(query, session_id)?;

        // 2. Search the raw index with the resolved query
        let hits = self
            .client
            .search_documents(&rewritten_query, top_k, workspace_root)
            .await?;

        // 3. Re-tokenize hits into session tokens (D3/I1 chokepoint)
        let retokenized = self
            .resolver
            .retokenize_hits(hits, session_id, workspace_root)
            .await?;

        Ok(RagSearchOutput { hits: retokenized })
    }
}

/// `RagSearchPort` adapter over [`RagSearchTool`] (D11).
///
/// The host constructs this at the composition root and injects it through
/// `ToolRuntimeHandles::with_rag_search_port`; `openbitfun-core` only ever sees
/// the port trait. The mutex serializes access because the pipeline mutates
/// the identity registry and session vault per search.
#[cfg(feature = "basemind")]
pub struct RagSearchPortAdapter {
    tool: tokio::sync::Mutex<RagSearchTool>,
}

#[cfg(feature = "basemind")]
impl RagSearchPortAdapter {
    pub fn new(tool: RagSearchTool) -> Self {
        Self {
            tool: tokio::sync::Mutex::new(tool),
        }
    }
}

#[cfg(feature = "basemind")]
#[async_trait::async_trait]
impl openbitfun_runtime_ports::RagSearchPort for RagSearchPortAdapter {
    async fn search(
        &self,
        request: openbitfun_runtime_ports::RagSearchRequest,
    ) -> Result<openbitfun_runtime_ports::RagSearchResponse, openbitfun_runtime_ports::RagSearchError>
    {
        if request.query.trim().is_empty() {
            return Err(openbitfun_runtime_ports::RagSearchError::new(
                openbitfun_runtime_ports::RagSearchErrorKind::InvalidRequest,
                "query must not be empty",
            ));
        }
        let mut tool = self.tool.lock().await;
        let output = tool
            .search(
                &request.query,
                request.top_k as usize,
                &request.session_id,
                &request.workspace_root,
            )
            .await
            .map_err(|e| {
                // D7: the pipeline hard-fails (session payload, sidecar, or
                // safety pass); the message names the failing stage.
                openbitfun_runtime_ports::RagSearchError::new(
                    openbitfun_runtime_ports::RagSearchErrorKind::Failed,
                    e.to_string(),
                )
            })?;

        Ok(openbitfun_runtime_ports::RagSearchResponse {
            hits: output
                .hits
                .into_iter()
                .map(|hit| openbitfun_runtime_ports::RagSearchHit {
                    chunk: hit.text,
                    source_doc: hit.source_doc,
                    score: hit.score,
                    entities: hit
                        .entities
                        .into_iter()
                        .map(|entity| openbitfun_runtime_ports::RagSearchEntity {
                            category: entity.category,
                            token: entity.token,
                        })
                        .collect(),
                })
                .collect(),
        })
    }
}
