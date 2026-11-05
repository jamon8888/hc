//! Document conversion — the single convergence point for agent file reads.
//!
//! Interception #2 of the basemind integration (SPEC §3): a document the
//! agent reads is extracted and redacted by the sidecar, so no unredacted
//! text reaches the model context (invariant I1).
//!
//! The sidecar reads the file itself, so the bytes the caller already read
//! are only used for the size guard; the extraction and the redaction both
//! happen in the fork's xberg pipeline.

#[cfg(feature = "basemind")]
use crate::ipc::BasemindClient;
#[cfg(feature = "basemind")]
use alex_runtime_ports::{
    DocumentRedactionError, DocumentRedactionErrorKind, DocumentRedactionPort, RedactedDocument,
};
#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use std::sync::Arc;

#[cfg(feature = "basemind")]
use crate::prompt_pseudonym::BasemindPromptPseudonymizer;

/// The product's document redaction port.
///
/// The sidecar redacts a document and returns the redacted text plus the
/// entities it found. The entity ids are the product's own, minted from a
/// hash of the real value (ADR 0001), so the same value keeps the same id
/// across documents and the user can reveal it from the dual-tab viewer.
#[cfg(feature = "basemind")]
pub struct BasemindDocumentRedactor {
    client: BasemindClient,
    pseudonymizer: Arc<BasemindPromptPseudonymizer>,
    workspace_root: String,
}

#[cfg(feature = "basemind")]
impl BasemindDocumentRedactor {
    pub fn new(
        client: BasemindClient,
        pseudonymizer: Arc<BasemindPromptPseudonymizer>,
        workspace_root: &std::path::Path,
    ) -> Self {
        Self {
            client,
            pseudonymizer,
            workspace_root: workspace_root.to_string_lossy().into_owned(),
        }
    }

    fn ensure_workspace_scope(&self, workspace_root: &str) -> Result<(), DocumentRedactionError> {
        if self.workspace_root != workspace_root {
            return Err(DocumentRedactionError::new(
                DocumentRedactionErrorKind::InvalidRequest,
                "document redaction refused: request workspace does not match the host privacy scope",
            ));
        }
        Ok(())
    }

    fn document_from_payload(
        payload: serde_json::Value,
        source_format: impl Into<String>,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        // The fork's NER coverage contract: a truncated pass means text past
        // the cut was never redacted. A document shown as fully redacted is
        // the one state the user must never get silently (D7).
        if let Some(message) = crate::redaction::ner_coverage_refusal(&payload) {
            return Err(DocumentRedactionError::new(
                DocumentRedactionErrorKind::Failed,
                message,
            ));
        }

        let rehydration = payload["rehydration_map"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let mut entities: Vec<alex_runtime_ports::RedactedEntity> = Vec::new();
        for (token, value) in &rehydration {
            let original = value.as_str().unwrap_or_default();
            if original.is_empty() {
                continue;
            }
            let category = category_of_token(token);
            let entity_id = entity_id_for_value(original, &category);
            match entities
                .iter_mut()
                .find(|entity| entity.entity_id == entity_id)
            {
                Some(entity) => entity.occurrence_count += 1,
                None => entities.push(alex_runtime_ports::RedactedEntity {
                    entity_id,
                    entity_type: category,
                    occurrence_count: 1,
                }),
            }
        }

        let markdown = payload["redacted_text"]
            .as_str()
            .ok_or_else(|| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::InvalidResponse,
                    "basemind redaction response has no redacted_text",
                )
            })?
            .to_string();

        Ok(RedactedDocument {
            markdown,
            source_format: source_format.into(),
            entities,
        })
    }

    /// OCR and redact an image attachment in a temporary file owned by the
    /// workspace host. Only session-bound text leaves this method.
    pub async fn redact_image_attachment_for_session(
        &self,
        mime_type: &str,
        data_base64: &str,
        workspace_root: &str,
        session_id: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        self.ensure_workspace_scope(workspace_root)?;
        let suffix = match mime_type {
            "image/png" => ".png",
            "image/jpeg" => ".jpg",
            "image/webp" => ".webp",
            "image/gif" => ".gif",
            _ => {
                return Err(DocumentRedactionError::new(
                    DocumentRedactionErrorKind::InvalidRequest,
                    format!("unsupported image attachment type: {mime_type}"),
                ));
            }
        };
        let encoded = data_base64
            .split_once(',')
            .map_or(data_base64, |(_, payload)| payload);
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::InvalidRequest,
                    format!("invalid image attachment encoding: {error}"),
                )
            })?;
        let temporary_path = std::path::Path::new(workspace_root).join(format!(
            ".alex-basemind-attachment-{}{}",
            uuid::Uuid::new_v4(),
            suffix
        ));
        let mut temporary = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Unavailable,
                    format!("could not create temporary image attachment: {error}"),
                )
            })?;
        use std::io::Write as _;
        if let Err(error) = temporary
            .write_all(&bytes)
            .and_then(|()| temporary.sync_all())
        {
            let _ = std::fs::remove_file(&temporary_path);
            return Err(DocumentRedactionError::new(
                DocumentRedactionErrorKind::Unavailable,
                format!("could not stage temporary image attachment: {error}"),
            ));
        }
        drop(temporary);

        let result = self
            .redact_for_session(
                &temporary_path.to_string_lossy(),
                workspace_root,
                session_id,
            )
            .await;
        let _ = std::fs::remove_file(&temporary_path);
        result
    }
}

#[cfg(feature = "basemind")]
#[async_trait::async_trait]
impl DocumentRedactionPort for BasemindDocumentRedactor {
    async fn redact(
        &self,
        path: &str,
        workspace_root: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        self.ensure_workspace_scope(workspace_root)?;
        let payload = self
            .client
            .redact_path(path, workspace_root)
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Unavailable,
                    format!("the basemind sidecar could not redact the document: {error}"),
                )
            })?;
        Self::document_from_payload(payload, source_format_for(path))
    }

    async fn redact_for_session(
        &self,
        path: &str,
        workspace_root: &str,
        session_id: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        self.ensure_workspace_scope(workspace_root)?;
        let payload = self
            .client
            .redact_path(path, workspace_root)
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Unavailable,
                    format!("the basemind sidecar could not redact the document: {error}"),
                )
            })?;
        if let Some(message) = crate::redaction::ner_coverage_refusal(&payload) {
            return Err(DocumentRedactionError::new(
                DocumentRedactionErrorKind::Failed,
                message,
            ));
        }
        self.pseudonymizer
            .retokenize_document(payload, session_id, source_format_for(path))
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Failed,
                    format!("the basemind document tokenization failed: {error}"),
                )
            })
    }

    async fn redact_text(
        &self,
        text: &str,
        workspace_root: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        self.ensure_workspace_scope(workspace_root)?;
        let payload = self
            .client
            .redact_text(text, workspace_root)
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Unavailable,
                    format!("the basemind sidecar could not redact inline text: {error}"),
                )
            })?;
        Self::document_from_payload(payload, "text")
    }

    async fn redact_text_for_session(
        &self,
        text: &str,
        workspace_root: &str,
        session_id: &str,
    ) -> Result<RedactedDocument, DocumentRedactionError> {
        self.ensure_workspace_scope(workspace_root)?;
        let payload = self
            .client
            .redact_text(text, workspace_root)
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Unavailable,
                    format!("the basemind sidecar could not redact inline text: {error}"),
                )
            })?;
        if let Some(message) = crate::redaction::ner_coverage_refusal(&payload) {
            return Err(DocumentRedactionError::new(
                DocumentRedactionErrorKind::Failed,
                message,
            ));
        }
        self.pseudonymizer
            .retokenize_document(payload, session_id, "text")
            .await
            .map_err(|error| {
                DocumentRedactionError::new(
                    DocumentRedactionErrorKind::Failed,
                    format!("the basemind document tokenization failed: {error}"),
                )
            })
    }
}

/// The format the sidecar extracted, from the file extension.
///
/// The sidecar's answer carries no format of its own, so the extension is
/// the only signal available.
#[cfg(feature = "basemind")]
fn source_format_for(path: &str) -> String {
    path.rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .filter(|extension| !extension.is_empty() && extension.len() <= 8)
        .unwrap_or_else(|| "unknown".to_string())
}

/// The category of a redaction token, e.g. `[EMAIL_1]` -> `EMAIL`.
#[cfg(feature = "basemind")]
fn category_of_token(token: &str) -> String {
    token
        .trim_start_matches('[')
        .trim_end_matches(']')
        .rsplit_once('_')
        .map(|(category, _)| category.to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

/// The entity id of a value, minted from a hash of the value itself.
///
/// The same value therefore carries the same id in every document, so the
/// user can tell that two mentions are the same person, and the value is
/// never readable from the id.
#[cfg(feature = "basemind")]
fn entity_id_for_value(value: &str, category: &str) -> String {
    let hash = crate::identity_registry::IdentityRegistry::compute_value_hash(value);
    format!(
        "ENT_{}_{}",
        category.to_uppercase(),
        &hash[..8.min(hash.len())]
    )
}

/// The sidecar's client for the running daemon.
///
/// The endpoint is discovered per call rather than cached: the daemon may
/// start after the product does, and a stale endpoint would fail closed for
/// a reason that has nothing to do with the request.
#[cfg(feature = "basemind")]
pub fn basemind_client() -> Result<BasemindClient> {
    let endpoint = crate::daemon::discover()?;
    Ok(BasemindClient::new(endpoint.base_url(), endpoint.token))
}
