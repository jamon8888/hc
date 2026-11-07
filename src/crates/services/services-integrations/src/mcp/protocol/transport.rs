//! MCP stdio transport runtime.

use super::{MCPError, MCPMessage, MCPNotification, MCPRequest, MCPResponse};
use crate::mcp::{MCPRuntimeError, MCPRuntimeResult};
use log::{debug, error, info, warn};
use serde_json::Value;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

type MCPWriter = Pin<Box<dyn AsyncWrite + Send>>;

pub struct MCPTransport {
    stdin: Arc<Mutex<MCPWriter>>,
    request_id: Arc<Mutex<u64>>,
    closed: Arc<AtomicBool>,
}

impl MCPTransport {
    pub fn new(stdin: ChildStdin) -> Self {
        Self::from_writer(Box::pin(stdin))
    }

    pub(crate) fn from_writer(stdin: MCPWriter) -> Self {
        Self {
            stdin: Arc::new(Mutex::new(stdin)),
            request_id: Arc::new(Mutex::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Permanently closes the writer after a cancelled write. A write can
    /// have reached the child partially, so reusing the stream would allow
    /// the next JSON-RPC request to be appended to a truncated message.
    pub async fn abort(&self) {
        self.closed.store(true, Ordering::Release);
        let mut stdin = self.stdin.lock().await;
        let _ = stdin.shutdown().await;
    }

    pub async fn next_request_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    pub async fn send_request(
        &self,
        method: String,
        params: Option<Value>,
    ) -> MCPRuntimeResult<u64> {
        let id = self.next_request_id().await;
        let request = MCPRequest::new(Value::Number(id.into()), method, params);
        self.send_message(MCPMessage::Request(request)).await?;
        Ok(id)
    }

    pub async fn send_request_with_id(
        &self,
        id: u64,
        method: String,
        params: Option<Value>,
    ) -> MCPRuntimeResult<()> {
        let request = MCPRequest::new(Value::Number(id.into()), method, params);
        self.send_message(MCPMessage::Request(request)).await
    }

    pub async fn send_notification(
        &self,
        method: String,
        params: Option<Value>,
    ) -> MCPRuntimeResult<()> {
        let notification = MCPNotification::new(method, params);
        self.send_message(MCPMessage::Notification(notification))
            .await
    }

    pub async fn send_response(&self, id: Value, result: Value) -> MCPRuntimeResult<()> {
        let response = MCPResponse::success(id, result);
        self.send_message(MCPMessage::Response(response)).await
    }

    pub async fn send_error(&self, id: Value, error: MCPError) -> MCPRuntimeResult<()> {
        let response = MCPResponse::error(id, error);
        self.send_message(MCPMessage::Response(response)).await
    }

    async fn send_message(&self, message: MCPMessage) -> MCPRuntimeResult<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(MCPRuntimeError::io(
                "MCP transport is closed after a cancelled write",
            ));
        }
        let json = serde_json::to_string(&message).map_err(|e| {
            MCPRuntimeError::serialization(format!("Failed to serialize MCP message: {}", e))
        })?;

        let mut stdin = self.stdin.lock().await;
        if self.closed.load(Ordering::Acquire) {
            return Err(MCPRuntimeError::io(
                "MCP transport is closed after a cancelled write",
            ));
        }
        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            MCPRuntimeError::io(format!("Failed to write to MCP server stdin: {}", e))
        })?;
        stdin.write_all(b"\n").await.map_err(|e| {
            MCPRuntimeError::io(format!(
                "Failed to write newline to MCP server stdin: {}",
                e
            ))
        })?;
        stdin
            .flush()
            .await
            .map_err(|e| MCPRuntimeError::io(format!("Failed to flush MCP server stdin: {}", e)))?;

        debug!("Sent MCP message: {}", json);
        Ok(())
    }

    pub fn start_receive_loop(stdout: ChildStdout, tx: mpsc::UnboundedSender<MCPMessage>) {
        Self::start_receive_loop_boxed(Box::pin(stdout), tx);
    }

    pub(crate) fn start_receive_loop_boxed(
        stdout: Pin<Box<dyn AsyncRead + Send>>,
        tx: mpsc::UnboundedSender<MCPMessage>,
    ) {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        info!("MCP server stdout closed");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<MCPMessage>(trimmed) {
                            Ok(message) => {
                                if tx.send(message).is_err() {
                                    warn!("Failed to send MCP message to handler: channel closed");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse MCP message: {} - Raw: {}", e, trimmed);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading from MCP server stdout: {}", e);
                        break;
                    }
                }
            }
        });
    }
}
