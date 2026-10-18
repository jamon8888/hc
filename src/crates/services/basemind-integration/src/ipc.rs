//! Basemind IPC client — stdio JSON-RPC transport for local and remote (SSH) sidecars.

#[cfg(feature = "basemind")]
use std::collections::HashMap;
#[cfg(feature = "basemind")]
use std::path::{Path, PathBuf};
#[cfg(feature = "basemind")]
use std::process::Stdio;
#[cfg(feature = "basemind")]
use std::sync::Arc;
#[cfg(feature = "basemind")]
use std::time::Duration;

#[cfg(feature = "basemind")]
use anyhow::{Context, Result};
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use serde_json::Value;
#[cfg(feature = "basemind")]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "basemind")]
use tokio::process::Command;
#[cfg(feature = "basemind")]
use tokio::sync::Mutex;
#[cfg(feature = "basemind")]
use tracing::{debug, warn};

use crate::models::{ExtractResult, ModelSpec, RagHit, PiiFinding};

/// Transport mode for Basemind communication.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub enum BasemindTransport {
    /// Local sidecar via stdio
    LocalStdio { child: Arc<Mutex<tokio::process::Child>> },
    /// In-process (for testing/embedded)
    LocalInProcess,
    /// Remote via SSH tunnel
    RemoteSsh { connection_id: String, remote_path: String },
}

/// JSON-RPC 2.0 request frame.
#[cfg(feature = "basemind")]
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC 2.0 response frame.
#[cfg(feature = "basemind")]
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
    id: u64,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// Errors from Basemind IPC.
#[cfg(feature = "basemind")]
#[derive(Debug, thiserror::Error)]
pub enum BasemindError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Timeout")]
    Timeout,
    #[error("Not initialized")]
    NotInitialized,
}

/// Basemind client with JSON-RPC over stdio/SSH.
#[cfg(feature = "basemind")]
pub struct BasemindClient {
    transport: BasemindTransport,
    request_id: Mutex<u64>,
    reader: Mutex<Option<BufReader<tokio::process::ChildStdout>>>,
    writer: Mutex<Option<tokio::process::ChildStdin>>,
    initialized: Mutex<bool>,
}

#[cfg(feature = "basemind")]
impl BasemindClient {
    /// Create an uninitialized client (for lazy initialization).
    pub fn new_uninitialized() -> Self {
        Self {
            transport: BasemindTransport::LocalInProcess,
            request_id: Mutex::new(0),
            reader: Mutex::new(None),
            writer: Mutex::new(None),
            initialized: Mutex::new(false),
        }
    }

    /// Spawn local Basemind sidecar and connect via stdio.
    pub async fn spawn_local(sidecar_path: Option<PathBuf>) -> Result<Self> {
        let sidecar = sidecar_path.unwrap_or_else(|| {
            // Default to resource directory
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../../../bin/basemind");
            #[cfg(target_os = "windows")]
            path.set_extension("exe");
            path
        });

        debug!("Spawning Basemind sidecar: {}", sidecar.display());

        let mut child = Command::new(&sidecar)
            .arg("serve")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn Basemind sidecar")?;

        let stdin = child.stdin.take().context("Failed to capture stdin")?;
        let stdout = child.stdout.take().context("Failed to capture stdout")?;

        let client = Self {
            transport: BasemindTransport::LocalStdio {
                child: Arc::new(Mutex::new(child)),
            },
            request_id: Mutex::new(0),
            reader: Mutex::new(Some(BufReader::new(stdout))),
            writer: Mutex::new(Some(stdin)),
            initialized: Mutex::new(false),
        };

        // Initialize the connection
        client.initialize().await?;

        Ok(client)
    }

    /// Connect to remote Basemind via SSH tunnel.
    pub async fn connect_remote(connection_id: String, remote_path: String) -> Result<Self> {
        // This would use OpenBitFun's SSH service to tunnel stdio
        // For now, return a placeholder that uses SSH command execution
        let client = Self {
            transport: BasemindTransport::RemoteSsh {
                connection_id,
                remote_path,
            },
            request_id: Mutex::new(0),
            reader: Mutex::new(None),
            writer: Mutex::new(None),
            initialized: Mutex::new(false),
        };

        client.initialize().await?;

        Ok(client)
    }

    /// Initialize the JSON-RPC connection (handshake).
    async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Ok(());
        }

        // Send initialize request
        let response = self.send_request("initialize", serde_json::json!({})).await?;
        
        if response.is_object() {
            *initialized = true;
            debug!("Basemind client initialized");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to initialize"))
        }
    }

    /// Send a JSON-RPC request and wait for response.
    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let mut id = self.request_id.lock().await;
        *id += 1;
        let request_id = *id;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id: request_id,
        };

        let request_json = serde_json::to_string(&request)?;
        debug!("Basemind RPC request: {}", request_json);

        match &self.transport {
            BasemindTransport::LocalStdio { .. } => {
                // Write to stdin
                let mut writer = self.writer.lock().await;
                if let Some(stdin) = writer.as_mut() {
                    stdin.write_all(request_json.as_bytes()).await?;
                    stdin.write_all(b"\n").await?;
                    stdin.flush().await?;
                }

                // Read response from stdout
                let mut reader = self.reader.lock().await;
                if let Some(stdout) = reader.as_mut() {
                    let mut line = String::new();
                    stdout.read_line(&mut line).await?;
                    let response: JsonRpcResponse = serde_json::from_str(&line)?;
                    
                    if let Some(error) = response.error {
                        return Err(anyhow::anyhow!("JSON-RPC error: Code {}: {}", error.code, error.message));
                    }
                    
                    Ok(response.result.unwrap_or(Value::Null))
                } else {
                    Err(anyhow::anyhow!("No stdout reader"))
                }
            }
            BasemindTransport::RemoteSsh { connection_id, remote_path } => {
                // Execute via SSH: `ssh <conn> basemind --stdio < request.json`
                // This is a simplified version; real implementation would use OpenBitFun's SSH service
                let ssh_cmd = format!(
                    "ssh {} 'cd {} && basemind serve --stdio'",
                    connection_id, remote_path
                );
                
                let _output = Command::new("sh")
                    .arg("-c")
                    .arg(&ssh_cmd)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(request_json.as_bytes())
                    .await?;

                // For now, return mock response
                Ok(Value::Null)
            }
            BasemindTransport::LocalInProcess => {
                Err(anyhow::anyhow!("Not initialized"))
            }
        }
    }

    /// Extract text and redact PII from document bytes.
    pub async fn extract_and_redact(&self, bytes: Vec<u8>, path_hint: String) -> Result<ExtractResult> {
        let params = serde_json::json!({
            "content": base64::encode(&bytes),
            "path_hint": path_hint,
        });

        let response = self.send_request("extract_and_redact", params).await?;
        
        Ok(ExtractResult {
            markdown: response["markdown"].as_str().unwrap_or("").to_string(),
            source_format: response["source_format"].as_str().unwrap_or("unknown").to_string(),
            entities: response["entities"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
                .unwrap_or_default(),
        })
    }

    /// Extract and redact from a file path.
    pub async fn extract_and_redact_path(&self, path: &str) -> Result<ExtractResult> {
        let params = serde_json::json!({
            "path": path,
        });

        let response = self.send_request("extract_and_redact_path", params).await?;
        
        Ok(ExtractResult {
            markdown: response["markdown"].as_str().unwrap_or("").to_string(),
            source_format: response["source_format"].as_str().unwrap_or("unknown").to_string(),
            entities: response["entities"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
                .unwrap_or_default(),
        })
    }

    /// Reveal the original value of a redacted entity.
    pub async fn reveal_entity(&self, entity_id: &str) -> Result<String> {
        let params = serde_json::json!({ "entity_id": entity_id });
        let response = self.send_request("reveal_entity", params).await?;
        
        Ok(response["value"].as_str().unwrap_or("").to_string())
    }

    /// Query RAG index.
    pub async fn query_rag(&self, query: &str, top_k: usize) -> Result<Vec<RagHit>> {
        let params = serde_json::json!({
            "query": query,
            "top_k": top_k,
        });

        let response = self.send_request("query_rag", params).await?;
        
        Ok(response["hits"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
            .unwrap_or_default())
    }

    /// Detect PII in text (for prompt pseudonymization).
    pub async fn detect_pii(&self, text: &str) -> Result<Vec<PiiFinding>> {
        let params = serde_json::json!({ "text": text });
        let response = self.send_request("detect_pii", params).await?;
        
        Ok(response["findings"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
            .unwrap_or_default())
    }

    /// Ensure models are downloaded.
    pub async fn ensure_models(&self, models: &[ModelSpec]) -> Result<()> {
        let params = serde_json::json!({
            "models": models,
        });
        self.send_request("ensure_models", params).await?;
        Ok(())
    }

    /// Check if client is initialized.
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.lock().await
    }
}

// Need to import base64
#[cfg(feature = "basemind")]
mod base64 {
    pub fn encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity(bytes.len() * 4 / 3 + 4);
        let mut i = 0;
        while i + 2 < bytes.len() {
            let b1 = bytes[i];
            let b2 = bytes[i + 1];
            let b3 = bytes[i + 2];
            result.push(TABLE[(b1 >> 2) as usize] as char);
            result.push(TABLE[((b1 & 0x03) << 4 | b2 >> 4) as usize] as char);
            result.push(TABLE[((b2 & 0x0F) << 2 | b3 >> 6) as usize] as char);
            result.push(TABLE[(b3 & 0x3F) as usize] as char);
            i += 3;
        }
        match bytes.len() - i {
            1 => {
                let b1 = bytes[i];
                result.push(TABLE[(b1 >> 2) as usize] as char);
                result.push(TABLE[((b1 & 0x03) << 4) as usize] as char);
                result.push('=');
                result.push('=');
            }
            2 => {
                let b1 = bytes[i];
                let b2 = bytes[i + 1];
                result.push(TABLE[(b1 >> 2) as usize] as char);
                result.push(TABLE[((b1 & 0x03) << 4 | b2 >> 4) as usize] as char);
                result.push(TABLE[((b2 & 0x0F) << 2) as usize] as char);
                result.push('=');
            }
            _ => {}
        }
        result
    }
}