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

use crate::models::ExtractResult;

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
}

#[cfg(feature = "basemind")]
impl BasemindClient {
    pub fn new(base_url: String, auth_token: String) -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base_url,
            auth_token,
        }
    }

    /// Call an MCP tool by name with JSON params.
    pub async fn call_tool(&self, tool: &str, params: Value) -> Result<Value> {
        let url = format!("{}/tools/call", self.base_url);
        let body = serde_json::json!({
            "name": tool,
            "arguments": params
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        Ok(resp)
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

    /// Extract a document and return redacted markdown with entity ids.
    ///
    /// Restored for the desktop redaction surface (`basemind_read_redacted_document`);
    /// the sidecar is now reached through MCP over HTTP instead of the legacy
    /// JSON-RPC transport. The payload is read from the MCP `content[0].json`
    /// envelope with a flat-response fallback.
    pub async fn extract_and_redact_path(&self, path: &str) -> Result<ExtractResult> {
        let response = self
            .call_tool(
                "extract_and_redact_path",
                serde_json::json!({ "path": path }),
            )
            .await?;
        let payload = response
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|item| item.get("json"))
            .cloned()
            .unwrap_or(response);
        Ok(ExtractResult {
            markdown: payload["markdown"].as_str().unwrap_or("").to_string(),
            source_format: payload["source_format"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            entities: payload["entities"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|value| serde_json::from_value(value.clone()).ok())
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// Reveal the original value of a redacted entity.
    ///
    /// Restored for the desktop reveal surface (`basemind_reveal_entity`); see
    /// `extract_and_redact_path` for the transport note.
    pub async fn reveal_entity(&self, entity_id: &str) -> Result<String> {
        let response = self
            .call_tool(
                "reveal_entity",
                serde_json::json!({ "entity_id": entity_id }),
            )
            .await?;
        let payload = response
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|item| item.get("json"))
            .cloned()
            .unwrap_or(response);
        Ok(payload["value"].as_str().unwrap_or("").to_string())
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

        // Parse the structured response
        let hits = result["content"][0]["json"]["hits"]
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
