//! Model management — lazy download, verification, cache for Basemind sidecar models.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use sha2::{Digest, Sha256};
#[cfg(feature = "basemind")]
use std::fs;
#[cfg(feature = "basemind")]
use std::io::{Cursor, Read};
#[cfg(feature = "basemind")]
use std::path::{Path, PathBuf};

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub sha256: String,
    pub model_type: ModelType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub archive_format: ModelArchiveFormat,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    NerGliner2,
    Embedding,
    Reranker,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelArchiveFormat {
    #[default]
    None,
    Zip,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    Missing,
    Invalid,
    Ready,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Serialize, Deserialize)]
struct ModelManifest {
    name: String,
    version: String,
    sha256: String,
    size_bytes: u64,
    archive_format: ModelArchiveFormat,
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
        if self.status(config) == ModelStatus::Ready {
            return Ok(model_path);
        }

        let model_parent = self.cache_dir.join(&config.name);
        fs::create_dir_all(&model_parent)?;
        let partial_path = model_parent.join(format!(
            ".{}.partial-{}",
            config.version,
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&partial_path)?;

        let result = self.download_and_install(config, &partial_path).await;
        match result {
            Ok(()) => {
                if model_path.exists() {
                    let quarantine = model_parent.join(format!(
                        ".{}.invalid-{}",
                        config.version,
                        uuid::Uuid::new_v4()
                    ));
                    fs::rename(&model_path, quarantine)?;
                }
                fs::rename(&partial_path, &model_path)?;
                Ok(model_path)
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&partial_path);
                Err(error)
            }
        }
    }

    pub fn status(&self, config: &ModelConfig) -> ModelStatus {
        let model_path = self.cache_dir.join(&config.name).join(&config.version);
        let manifest_path = model_path.join("manifest.json");
        let source_path = model_path.join("source.bin");
        let Ok(manifest_text) = fs::read_to_string(manifest_path) else {
            return if model_path.exists() {
                ModelStatus::Invalid
            } else {
                ModelStatus::Missing
            };
        };
        let Ok(manifest) = serde_json::from_str::<ModelManifest>(&manifest_text) else {
            return ModelStatus::Invalid;
        };
        if manifest.name != config.name
            || manifest.version != config.version
            || !manifest.sha256.eq_ignore_ascii_case(&config.sha256)
            || manifest.archive_format != config.archive_format
            || config
                .size_bytes
                .is_some_and(|expected| manifest.size_bytes != expected)
            || !source_path.is_file()
        {
            return ModelStatus::Invalid;
        }
        match sha256_file(&source_path) {
            Ok(digest) if digest.eq_ignore_ascii_case(&config.sha256) => ModelStatus::Ready,
            _ => ModelStatus::Invalid,
        }
    }

    async fn download_and_install(
        &self,
        config: &ModelConfig,
        destination: &Path,
    ) -> Result<()> {
        let response = self
            .client
            .get(&config.download_url)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;
        if let Some(expected_size) = config.size_bytes {
            if bytes.len() as u64 != expected_size {
                anyhow::bail!(
                    "model {} has size {}, expected {}",
                    config.name,
                    bytes.len(),
                    expected_size
                );
            }
        }
        let digest = sha256_bytes(&bytes);
        if !digest.eq_ignore_ascii_case(&config.sha256) {
            anyhow::bail!(
                "model {} sha256 mismatch: got {}, expected {}",
                config.name,
                digest,
                config.sha256
            );
        }

        fs::write(destination.join("source.bin"), &bytes)?;
        match config.archive_format {
            ModelArchiveFormat::None => {
                fs::write(destination.join("model.bin"), &bytes)?;
            }
            ModelArchiveFormat::Zip => extract_zip(&bytes, destination)?,
        }
        let manifest = ModelManifest {
            name: config.name.clone(),
            version: config.version.clone(),
            sha256: digest,
            size_bytes: bytes.len() as u64,
            archive_format: config.archive_format.clone(),
        };
        fs::write(
            destination.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(())
    }
}

#[cfg(feature = "basemind")]
fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(feature = "basemind")]
fn sha256_file(path: &Path) -> Result<String> {
    Ok(sha256_bytes(&fs::read(path)?))
}

#[cfg(feature = "basemind")]
fn extract_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| anyhow::anyhow!("model archive contains an unsafe path"))?
            .to_path_buf();
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        fs::write(output, contents)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "basemind"))]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn missing_and_invalid_cache_states_are_explicit() {
        let directory = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(directory.path().to_path_buf());
        let config = ModelConfig {
            name: "ner".into(),
            version: "1".into(),
            download_url: "https://example.invalid/ner.zip".into(),
            sha256: "00".repeat(32),
            model_type: ModelType::NerGliner2,
            required: true,
            size_bytes: None,
            archive_format: ModelArchiveFormat::Zip,
        };
        assert_eq!(manager.status(&config), ModelStatus::Missing);
        let path = directory.path().join("ner").join("1");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("manifest.json"), "{}").unwrap();
        assert_eq!(manager.status(&config), ModelStatus::Invalid);
    }

    #[test]
    fn sha256_helper_matches_known_payload() {
        let mut hasher = Sha256::new();
        hasher.update(b"model");
        assert_eq!(sha256_bytes(b"model"), hex::encode(hasher.finalize()));
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
