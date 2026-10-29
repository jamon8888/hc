//! PII-safe document redaction port.
//!
//! The only shape `alex-core` ever sees for document extraction
//! (SPEC §3, invariant I1): a document the agent reads is extracted and
//! redacted by the sidecar, so no unredacted text reaches the model context.
//!
//! The concrete pipeline lives in `alex-basemind-integration` and is
//! published by the host at the composition root, the same way the
//! `RagSearchPort` is published (D11).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A personal value found in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedEntity {
    /// Stable id for the entity, minted by the product from a hash of the
    /// value, so the same value keeps the same id across documents.
    pub entity_id: String,
    /// The category of the value (`PERSON`, `IBAN`, ...).
    pub entity_type: String,
    /// How many times the value occurs in the document.
    pub occurrence_count: u32,
}

/// A redacted document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedDocument {
    /// The redacted text: the document with every detected value replaced by
    /// an opaque token.
    pub markdown: String,
    /// The source format the sidecar extracted (`pdf`, `docx`, ...).
    pub source_format: String,
    /// The entities the sidecar found.
    pub entities: Vec<RedactedEntity>,
}

/// Why a redaction failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRedactionErrorKind {
    /// The document is outside the workspace, or is not a file.
    InvalidRequest,
    /// The sidecar could not be reached.
    Unavailable,
    /// The sidecar answered with something the product cannot use.
    InvalidResponse,
    /// The redaction failed.
    Failed,
}

/// A redaction failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRedactionError {
    pub kind: DocumentRedactionErrorKind,
    pub message: String,
}

impl DocumentRedactionError {
    pub fn new(kind: DocumentRedactionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for DocumentRedactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DocumentRedactionError {}

/// Extracts and redacts a document.
#[async_trait]
pub trait DocumentRedactionPort: Send + Sync {
    /// Redact the document at `path`, hosted under `workspace_root`.
    ///
    /// The sidecar reads the file itself, so `path` must be inside
    /// `workspace_root`: the sidecar only serves paths under the root it was
    /// asked to host.
    async fn redact(
        &self,
        path: &str,
        workspace_root: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError>;
}
