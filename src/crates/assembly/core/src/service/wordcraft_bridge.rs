//! Request/response bridge for commands issued by the Agent to an embedded
//! WordCraft surface.
//!
//! The bridge is deliberately surface-neutral: Core owns the pending request
//! lifecycle and emits a typed backend event, while the desktop/Web UI adapter
//! delivers the request to the visible iframe and completes it through the
//! command below. A missing or disconnected surface therefore fails the tool
//! call instead of falling back to a second headless writer.

use crate::infrastructure::events::{emit_global_event, BackendEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::{oneshot, Mutex};
use tokio::time::{error::Elapsed, timeout, Duration};

pub const WORDCRAFT_COMMAND_REQUEST_EVENT: &str = "backend-event-wordcraftcommandrequest";
const WORDCRAFT_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftEmbeddedCommandRequest {
    pub request_id: String,
    pub document_id: String,
    pub session_id: String,
    pub surface_id: String,
    pub workspace_id: String,
    pub file_path: String,
    pub expected_revision: u64,
    pub operation: String,
    pub params: Value,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftEmbeddedCommandResponse {
    pub request_id: String,
    pub ok: bool,
    /// Optional for upgrade compatibility with older Web UI hosts. New hosts
    /// echo the request identity so Core can reject stale cross-document
    /// completions before consuming the pending request.
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub revision: Option<u64>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug)]
struct PendingResult {
    document_id: String,
    session_id: String,
    expected_revision: u64,
    sender: oneshot::Sender<Result<Value, String>>,
}

#[derive(Debug, Default)]
pub struct WordcraftBridge {
    pending: Mutex<HashMap<String, PendingResult>>,
}

impl WordcraftBridge {
    pub async fn request(&self, request: WordcraftEmbeddedCommandRequest) -> Result<Value, String> {
        validate_request(&request)?;
        let request_id = request.request_id.clone();
        let (sender, receiver) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            if pending
                .insert(
                    request_id.clone(),
                    PendingResult {
                        document_id: request.document_id.clone(),
                        session_id: request.session_id.clone(),
                        expected_revision: request.expected_revision,
                        sender,
                    },
                )
                .is_some()
            {
                return Err(format!(
                    "WordCraft request ID is already pending: {request_id}"
                ));
            }
        }

        if let Err(error) = emit_global_event(BackendEvent::Custom {
            event_name: WORDCRAFT_COMMAND_REQUEST_EVENT.to_string(),
            payload: serde_json::to_value(&request)
                .map_err(|error| format!("failed to serialize WordCraft request: {error}"))?,
        })
        .await
        {
            self.pending.lock().await.remove(&request_id);
            return Err(format!("failed to deliver WordCraft request: {error}"));
        }

        match timeout(WORDCRAFT_COMMAND_TIMEOUT, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("WordCraft command bridge was closed".to_string()),
            Err(Elapsed { .. }) => {
                self.pending.lock().await.remove(&request_id);
                Err(format!(
                    "WordCraft command '{}' timed out",
                    request.operation
                ))
            }
        }
    }

    pub async fn complete(&self, response: WordcraftEmbeddedCommandResponse) -> Result<(), String> {
        let mut pending = self.pending.lock().await;
        let expected = pending.get(&response.request_id).ok_or_else(|| {
            format!(
                "WordCraft request is not pending or has already timed out: {}",
                response.request_id
            )
        })?;

        if response
            .document_id
            .as_deref()
            .is_some_and(|document_id| document_id != expected.document_id)
        {
            return Err("WordCraft response document identity is stale".to_string());
        }
        if response
            .session_id
            .as_deref()
            .is_some_and(|session_id| session_id != expected.session_id)
        {
            return Err("WordCraft response session identity is stale".to_string());
        }
        if response
            .revision
            .is_some_and(|revision| revision < expected.expected_revision)
        {
            return Err("WordCraft response revision is stale".to_string());
        }

        let pending = pending.remove(&response.request_id).ok_or_else(|| {
            format!(
                "WordCraft request is not pending or has already timed out: {}",
                response.request_id
            )
        })?;

        let result = if response.ok {
            Ok(response.result.unwrap_or(Value::Null))
        } else {
            Err(response
                .error
                .unwrap_or_else(|| "WordCraft command failed without an error".to_string()))
        };
        pending
            .sender
            .send(result)
            .map_err(|_| "WordCraft command caller was dropped".to_string())
    }
}

fn validate_request(request: &WordcraftEmbeddedCommandRequest) -> Result<(), String> {
    for (name, value) in [
        ("request_id", request.request_id.as_str()),
        ("document_id", request.document_id.as_str()),
        ("session_id", request.session_id.as_str()),
        ("workspace_id", request.workspace_id.as_str()),
        ("file_path", request.file_path.as_str()),
        ("operation", request.operation.as_str()),
        ("author", request.author.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("WordCraft request field '{name}' is empty"));
        }
    }
    if !request.params.is_object() {
        return Err("WordCraft request params must be an object".to_string());
    }
    Ok(())
}

static GLOBAL_WORDCRAFT_BRIDGE: OnceLock<Arc<WordcraftBridge>> = OnceLock::new();

pub fn get_global_wordcraft_bridge() -> Arc<WordcraftBridge> {
    GLOBAL_WORDCRAFT_BRIDGE
        .get_or_init(|| Arc::new(WordcraftBridge::default()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::{
        WordcraftBridge, WordcraftEmbeddedCommandRequest, WordcraftEmbeddedCommandResponse,
    };
    use serde_json::json;
    use tokio::time::{timeout, Duration};

    fn request(id: &str) -> WordcraftEmbeddedCommandRequest {
        WordcraftEmbeddedCommandRequest {
            request_id: id.to_string(),
            document_id: "document-1".to_string(),
            session_id: "surface-1".to_string(),
            surface_id: "local".to_string(),
            workspace_id: "workspace-1".to_string(),
            file_path: "notes.docx".to_string(),
            expected_revision: 2,
            operation: "get_text".to_string(),
            params: json!({}),
            author: "Alex Agent (agent-1)".to_string(),
        }
    }

    #[tokio::test]
    async fn completion_resolves_the_matching_request() {
        let bridge = std::sync::Arc::new(WordcraftBridge::default());
        let task_bridge = bridge.clone();
        let task = tokio::spawn(async move { task_bridge.request(request("request-1")).await });
        tokio::task::yield_now().await;

        bridge
            .complete(WordcraftEmbeddedCommandResponse {
                request_id: "request-1".to_string(),
                ok: true,
                document_id: Some("document-1".to_string()),
                session_id: Some("surface-1".to_string()),
                revision: Some(2),
                result: Some(json!({"text": "hello"})),
                error: None,
            })
            .await
            .expect("completion should be accepted");

        let result = timeout(Duration::from_secs(1), task)
            .await
            .expect("request should complete")
            .expect("request task should not panic")
            .expect("request should succeed");
        assert_eq!(result, json!({"text": "hello"}));
    }

    #[tokio::test]
    async fn completion_rejects_unknown_request_ids() {
        let bridge = WordcraftBridge::default();
        let error = bridge
            .complete(WordcraftEmbeddedCommandResponse {
                request_id: "missing".to_string(),
                ok: false,
                document_id: None,
                session_id: None,
                revision: None,
                result: None,
                error: Some("closed".to_string()),
            })
            .await
            .expect_err("unknown requests must fail closed");
        assert!(error.contains("not pending"));
    }

    #[tokio::test]
    async fn stale_completion_does_not_consume_the_pending_request() {
        let bridge = std::sync::Arc::new(WordcraftBridge::default());
        let task_bridge = bridge.clone();
        let task = tokio::spawn(async move { task_bridge.request(request("request-2")).await });
        tokio::task::yield_now().await;

        let error = bridge
            .complete(WordcraftEmbeddedCommandResponse {
                request_id: "request-2".to_string(),
                ok: true,
                document_id: Some("other-document".to_string()),
                session_id: Some("surface-1".to_string()),
                revision: Some(2),
                result: Some(json!({"text": "stale"})),
                error: None,
            })
            .await
            .expect_err("stale completion must be rejected");
        assert!(error.contains("document identity"));

        bridge
            .complete(WordcraftEmbeddedCommandResponse {
                request_id: "request-2".to_string(),
                ok: true,
                document_id: Some("document-1".to_string()),
                session_id: Some("surface-1".to_string()),
                revision: Some(2),
                result: Some(json!({"text": "current"})),
                error: None,
            })
            .await
            .expect("current completion should still be accepted");

        let result = timeout(Duration::from_secs(1), task)
            .await
            .expect("request should complete")
            .expect("request task should not panic")
            .expect("request should succeed");
        assert_eq!(result, json!({"text": "current"}));
    }
}
