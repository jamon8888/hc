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
    /// The snapshot's endpoint values, used only by `Snapshot` mode.
    base_url: String,
    auth_token: String,
    /// How this client reaches the daemon: a snapshot taken once from the
    /// portfile, or a living client that re-discovers (and, when the sidecar
    /// is known, re-ensures) the daemon on every call, because the daemon
    /// may restart with a fresh portfile between calls.
    living: EndpointSource,
    /// The fail-closed redaction policy every sidecar redaction call obeys.
    redaction: RedactionPolicy,
    next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(feature = "basemind")]
#[derive(Clone)]
enum EndpointSource {
    Snapshot,
    Living {
        daemon_binary: Option<std::path::PathBuf>,
    },
}

/// The NER obligations the product puts on every sidecar redaction call.
///
/// The sidecar degrades to pattern-only redaction when NER cannot run, so a
/// product that promises that no unredacted personal value reaches a model
/// must pass `require_ner` and point the sidecar at its bundled GLiNER2
/// model. The pattern detectors (`email`, `iban`, ...) run regardless; NER
/// is what catches names, addresses and the PII taxonomy's model-found
/// spans.
#[cfg(feature = "basemind")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedactionPolicy {
    /// Refuse a redaction whose NER run did not happen, instead of
    /// answering with pattern-only output.
    pub require_ner: bool,
    /// The directory holding the pinned GLiNER2 model (`tokenizer.json`,
    /// `model.safetensors`, `encoder_config/config.json`) the daemon uses.
    pub ner_model_dir: Option<String>,
}

#[cfg(feature = "basemind")]
impl RedactionPolicy {
    /// The policy a product that ships the bundled model installs: NER is
    /// required, and the sidecar is told where the model lives.
    pub fn bundled(model_dir: std::path::PathBuf) -> Self {
        Self {
            require_ner: true,
            ner_model_dir: Some(model_dir.to_string_lossy().to_string()),
        }
    }
}

#[cfg(feature = "basemind")]
impl BasemindClient {
    /// Create a client for a running basemind daemon.
    ///
    /// `base_url` is the daemon's `host:port` as published in the portfile
    /// (`<comms_dir>/http.addr`, line 1); `auth_token` is the bearer token on
    /// line 2.
    ///
    /// The client is the connection, not a workspace: one daemon hosts many
    /// workspaces, so the root travels with each call.
    pub fn new(base_url: String, auth_token: String) -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base_url,
            auth_token,
            living: EndpointSource::Snapshot,
            redaction: RedactionPolicy::default(),
            next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Create a client that follows the daemon instead of a snapshot.
    ///
    /// The daemon may be self-terminated after its bootstrap window when a
    /// client snapshot was taken at startup, and it binds a fresh port on
    /// every restart, so a composition root that owns the sidecar hands the
    /// client the binary the daemon could be brought back from. Each call
    /// re-reads the portfile, and respawns the daemon when no endpoint is
    /// published. Without a binary the client still re-discovers on every
    /// call; it simply cannot bring a dead daemon back.
    pub fn lazy(daemon_binary: Option<std::path::PathBuf>) -> Self {
        let client = Self::new(String::new(), String::new());
        Self {
            living: EndpointSource::Living { daemon_binary },
            ..client
        }
    }

    /// Install the fail-closed redaction policy for this client.
    pub fn with_redaction_policy(mut self, policy: RedactionPolicy) -> Self {
        self.redaction = policy;
        self
    }

    /// Put the policy's NER obligations on a redaction call's arguments.
    ///
    /// Only sidecar redaction tools accept these arguments; other tools are
    /// not called with them.
    fn apply_redaction_policy(&self, arguments: &mut Value) {
        if self.redaction.require_ner {
            arguments["require_ner"] = Value::Bool(true);
        }
        if let Some(ner_model_dir) = &self.redaction.ner_model_dir {
            arguments["ner_model_dir"] = Value::String(ner_model_dir.clone());
        }
    }

    /// The endpoint to use for this call, ensured to be alive.
    async fn endpoint(&self) -> Result<(String, String)> {
        match &self.living {
            EndpointSource::Snapshot => Ok((self.base_url.clone(), self.auth_token.clone())),
            EndpointSource::Living { daemon_binary } => {
                let endpoint = crate::daemon::discover_or_ensure(daemon_binary.clone(), 15).await?;
                Ok((endpoint.base_url(), endpoint.token))
            }
        }
    }

    /// Call an MCP tool by name with JSON params.
    ///
    /// The daemon speaks MCP over streamable HTTP: a JSON-RPC 2.0 request
    /// posted to `/mcp` with the workspace root in the query string, answered
    /// with a JSON-RPC envelope whose `result` carries the tool result.
    /// Returns the `structuredContent` payload when the tool provides one
    /// and falls back to the first text content block.
    pub async fn call_tool(&self, tool: &str, params: Value, root: &str) -> Result<Value> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (base_url, auth_token) = self.endpoint().await?;
        let url = format!("{}/mcp?root={}", base_url, urlencoding(root));
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
            .bearer_auth(&auth_token)
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
    pub async fn redact_text(&self, text: &str, root: &str) -> Result<Value> {
        let mut arguments = serde_json::json!({
            "text": text,
            "strategy": "token-replace"
        });
        self.apply_redaction_policy(&mut arguments);
        self.call_tool("redact_text", arguments, root).await
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
    pub async fn redact_path(&self, path: &str, root: &str) -> Result<Value> {
        let mut arguments = serde_json::json!({
            "file_path": path,
            "strategy": "token-replace",
        });
        self.apply_redaction_policy(&mut arguments);
        self.call_tool("redact_text", arguments, root).await
    }

    /// Convenience: memory search documents
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
        root: &str,
    ) -> Result<Vec<DocumentSearchHit>> {
        let result = self
            .call_tool(
                "memory",
                serde_json::json!({
                    "mode": "documents",
                    "query": query,
                    "limit": limit
                }),
                root,
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
    pub async fn vault_encrypt(&self, map: Value, passphrase: &str, root: &str) -> Result<Value> {
        self.call_tool(
            "vault",
            serde_json::json!({
                "mode": "encrypt",
                "map": map,
                "passphrase": passphrase
            }),
            root,
        )
        .await
    }

    pub async fn vault_decrypt(
        &self,
        encrypted_blob: &str,
        passphrase: &str,
        root: &str,
    ) -> Result<Value> {
        self.call_tool(
            "vault",
            serde_json::json!({
                "mode": "decrypt",
                "encrypted_blob": encrypted_blob,
                "passphrase": passphrase
            }),
            root,
        )
        .await
    }
}
