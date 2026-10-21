//! BasemindClient — MCP over HTTP client for the Basemind sidecar.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use reqwest::Client as HttpClient;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use serde_json::Value;
#[cfg(feature = "basemind")]
use std::time::Duration;

/// Percent-encode a value for use in a query string.
///
/// The daemon decodes the query with `form_urlencoded`, so the encoding must
/// be the same one: a path with spaces or non-ASCII characters otherwise
/// changes the query the daemon parses.
#[cfg(feature = "basemind")]
pub fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Pull the tool payload out of an MCP result envelope.
///
/// A tool that returns structured content puts it in `structuredContent`; a
/// tool that returns only text puts the JSON document in the first content
/// block. Prefer the structured form and fall back to parsing the text.
#[cfg(feature = "basemind")]
pub fn extract_tool_payload(result: &Value) -> Value {
    if let Some(structured) = result.get("structuredContent") {
        if !structured.is_null() {
            return structured.clone();
        }
    }
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| blocks.first())
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| result.clone())
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntity {
    pub category: String,
    pub text: String,
    pub start: u32,
    pub end: u32,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchHit {
    pub path: String,
    pub chunk_idx: u32,
    pub text: String,
    pub mime_type: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub distance: f32,
    #[serde(default)]
    pub rerank_score: Option<f32>,
    #[serde(default)]
    pub keywords: Vec<DocKeyword>,
    #[serde(default)]
    pub entities: Vec<DocEntity>,
    #[serde(default)]
    pub summary: Option<DocSummary>,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocKeyword {
    pub keyword: String,
    pub score: f32,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSummary {
    pub text: String,
    pub confidence: f32,
}

#[cfg(feature = "basemind")]
#[derive(Clone)]
pub struct BasemindClient {
    http: HttpClient,
    base_url: String,
    auth_token: String,
    root: String,
    next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(feature = "basemind")]
impl BasemindClient {
    /// Create a client for a running basemind daemon.
    ///
    /// `base_url` is the daemon's `host:port` as published in the portfile
    /// (`<comms_dir>/http.addr`, line 1); `auth_token` is the bearer token on
    /// line 2. `root` is the absolute workspace root the daemon should host:
    /// the daemon refuses any root that is neither a git repository nor a
    /// directory carrying a basemind config.
    pub fn new(base_url: String, auth_token: String, root: String) -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base_url,
            auth_token,
            root,
            next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Call an MCP tool by name with JSON params.
    ///
    /// The daemon speaks MCP over streamable HTTP: a JSON-RPC 2.0 request
    /// posted to `/mcp` with the workspace root in the query string, answered
    /// with a JSON-RPC envelope whose `result` carries the tool result.
    /// Returns the `structuredContent` payload when the tool provides one
    /// and falls back to the first text content block.
    pub async fn call_tool(&self, tool: &str, params: Value) -> Result<Value> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let url = format!("{}/mcp?root={}", self.base_url, urlencoding(&self.root));
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": params,
            }
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.auth_token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(error) = resp.get("error") {
            anyhow::bail!("basemind tool {tool} failed: {error}");
        }
        let result = resp
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("basemind tool {tool} returned no result"))?;
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            anyhow::bail!("basemind tool {tool} reported an error: {result}");
        }
        Ok(extract_tool_payload(result))
    }

    /// Convenience: redact_text tool
    pub async fn redact_text(&self, text: &str) -> Result<Value> {
        self.call_tool(
            "redact_text",
            serde_json::json!({
                "text": text,
                "strategy": "token-replace"
            }),
        )
        .await
    }

    /// Extract a document and return the fork's redaction payload.
    ///
    /// The fork exposes document extraction and redaction as one MCP tool:
    /// `redact_text` takes either inline `text` or a `file_path` it extracts
    /// itself (PDF, Office, HTML, images via OCR) and answers with
    /// `redacted_text`, the `rehydration_map` (token -> original value) and
    /// the `detections` it found.
    ///
    /// The returned payload is the fork's own answer; turning it into
    /// entity ids and a revealable document is the product's job
    /// (`crate::redaction`).
    pub async fn redact_path(&self, path: &str) -> Result<Value> {
        self.call_tool(
            "redact_text",
            serde_json::json!({
                "file_path": path,
                "strategy": "token-replace",
            }),
        )
        .await
    }

    /// Convenience: memory search documents
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DocumentSearchHit>> {
        let result = self
            .call_tool(
                "memory",
                serde_json::json!({
                    "mode": "documents",
                    "query": query,
                    "limit": limit
                }),
            )
            .await?;

        // `call_tool` already unwrapped the MCP envelope, so the hits sit
        // directly in the payload.
        let hits = result["hits"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing hits array"))?;

        let mut parsed = Vec::new();
        for hit in hits {
            if let Ok(doc_hit) = serde_json::from_value::<DocumentSearchHit>(hit.clone()) {
                parsed.push(doc_hit);
            }
        }
        Ok(parsed)
    }

    /// Convenience: vault operations
    pub async fn vault_encrypt(&self, map: Value, passphrase: &str) -> Result<Value> {
        self.call_tool(
            "vault",
            serde_json::json!({
                "mode": "encrypt",
                "map": map,
                "passphrase": passphrase
            }),
        )
        .await
    }

    pub async fn vault_decrypt(&self, encrypted_blob: &str, passphrase: &str) -> Result<Value> {
        self.call_tool(
            "vault",
            serde_json::json!({
                "mode": "decrypt",
                "encrypted_blob": encrypted_blob,
                "passphrase": passphrase
            }),
        )
        .await
    }
}
