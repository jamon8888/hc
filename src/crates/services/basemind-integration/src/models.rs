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
