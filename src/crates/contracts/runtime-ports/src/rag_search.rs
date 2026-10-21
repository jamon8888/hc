//! PII-safe workspace RAG search port (D11).
//!
//! The only RAG shape `openbitfun-core` ever sees: the concrete pipeline lives
//! in `openbitfun-basemind-integration` behind `ToolRuntimeHandles` injection.
//! Hits are session-tokenized at the LLM boundary; the underlying values never
//! cross this interface (D3/D4, SPEC §4).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Request for a PII-safe workspace RAG search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchRequest {
    /// The query as the model wrote it (session tokens included); the port
    /// resolves echoed tokens through the session vault before searching.
    pub query: String,
    pub top_k: u32,
    /// The session whose vault payload scopes token resolution and minting.
    pub session_id: String,
    /// The absolute workspace root the sidecar hosts.
    ///
    /// One daemon hosts many workspaces, so the root travels with the
    /// request; the sidecar refuses any root that is neither a git
    /// repository nor a directory carrying a basemind config.
    pub workspace_root: String,
}

/// A rehydratable entity reference: category plus opaque session token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchEntity {
    pub category: String,
    /// Opaque session token `[CAT_<H16>]`; the underlying value lives only in
    /// the session vault (D3/D4).
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchHit {
    pub chunk: String,
    pub source_doc: String,
    pub score: f32,
    pub entities: Vec<RagSearchEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchResponse {
    pub hits: Vec<RagSearchHit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RagSearchErrorKind {
    InvalidRequest,
    /// The session vault payload (salt + map) could not be loaded; the port
    /// hard-fails rather than returning raw text (D7).
    SessionUnavailable,
    Transport,
    InvalidResponse,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchError {
    pub kind: RagSearchErrorKind,
    pub message: String,
}

impl RagSearchError {
    pub fn new(kind: RagSearchErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RagSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RAG search {:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for RagSearchError {}

#[async_trait]
pub trait RagSearchPort: Send + Sync {
    async fn search(&self, request: RagSearchRequest) -> Result<RagSearchResponse, RagSearchError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_response_wire_shapes_are_stable() {
        let request = RagSearchRequest {
            query: "[PERSON_9c21de04a7f3b2d1]'s chronology".to_string(),
            top_k: 8,
            session_id: "session-1".to_string(),
            workspace_root: "/srv/workspace".to_string(),
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["query"], "[PERSON_9c21de04a7f3b2d1]'s chronology");
        assert_eq!(value["topK"], 8);
        assert_eq!(value["sessionId"], "session-1");

        let response = RagSearchResponse {
            hits: vec![RagSearchHit {
                chunk: "…[PERSON_9c21de04a7f3b2d1] joined Acme in 2019…".to_string(),
                source_doc: "notes/meeting.md".to_string(),
                score: 0.82,
                entities: vec![RagSearchEntity {
                    category: "person".to_string(),
                    token: "[PERSON_9c21de04a7f3b2d1]".to_string(),
                }],
            }],
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["hits"][0]["sourceDoc"], "notes/meeting.md");
        assert_eq!(
            value["hits"][0]["entities"][0]["token"],
            "[PERSON_9c21de04a7f3b2d1]"
        );
    }
}
