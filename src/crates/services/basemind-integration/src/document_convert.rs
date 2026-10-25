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
use anyhow::Result;
#[cfg(feature = "basemind")]
use openbitfun_runtime_ports::{
    DocumentRedactionError, DocumentRedactionErrorKind, DocumentRedactionPort, RedactedDocument,
};

/// The product's document redaction port.
///
/// The sidecar redacts a document and returns the redacted text plus the
/// entities it found. The entity ids are the product's own, minted from a
/// hash of the real value (ADR 0001), so the same value keeps the same id
/// across documents and the user can reveal it from the dual-tab viewer.
#[cfg(feature = "basemind")]
pub struct BasemindDocumentRedactor {
    client: BasemindClient,
}

#[cfg(feature = "basemind")]
impl BasemindDocumentRedactor {
    pub fn new(client: BasemindClient) -> Self {
        Self { client }
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

        // The fork's NER coverage contract: a truncated pass means text past
        // the cut was never redacted. A document shown as fully redacted is
        // the one state the user must never get silently (D7), so this
        // refusal keeps the failure loud on the surface the user sees.
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
        let mut entities: Vec<openbitfun_runtime_ports::RedactedEntity> = Vec::new();
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
                None => entities.push(openbitfun_runtime_ports::RedactedEntity {
                    entity_id,
                    entity_type: category,
                    occurrence_count: 1,
                }),
            }
        }

        Ok(RedactedDocument {
            markdown: payload["redacted_text"].as_str().unwrap_or("").to_string(),
            source_format: source_format_for(path),
            entities,
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
