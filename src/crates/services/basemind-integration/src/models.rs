//! Model management — lazy download, verification, cache for Basemind sidecar models.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use std::path::PathBuf;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub sha256: String,
    pub model_type: ModelType,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    NerGliner2,
    Embedding,
    Reranker,
}

#[cfg(feature = "basemind")]
pub struct ModelManager {
    cache_dir: PathBuf,
    client: reqwest::Client,
}

#[cfg(feature = "basemind")]
impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Self { cache_dir, client }
    }

    pub async fn ensure_model(&self, config: &ModelConfig) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(&config.name).join(&config.version);
        if model_path.exists() {
            return Ok(model_path);
        }
        // Download and verify
        todo!(
            "Download model from {} and verify sha256",
            config.download_url
        );
    }
}

/// Extract result from the Basemind sidecar.
///
/// Compatibility shape restored for the desktop redaction surface
/// (`basemind_read_redacted_document`), which predates the identity spine.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub markdown: String,
    pub source_format: String,
    pub entities: Vec<RedactedEntity>,
}

/// PII entity found during extraction, referenced by id.
///
/// Compatibility shape restored for the desktop redaction surface
/// (`basemind_reveal_entity` / `basemind_read_redacted_document`).
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub occurrence_count: u32,
}
