//! Model management — lazy download, verification, cache for Basemind sidecar models.

#[cfg(feature = "basemind")]
use alex_config_contracts::BasemindConfig;
#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use sha2::{Digest, Sha256};
#[cfg(feature = "basemind")]
use std::collections::BTreeSet;
#[cfg(feature = "basemind")]
use std::fs;
#[cfg(feature = "basemind")]
use std::io::{Read, Write};
#[cfg(feature = "basemind")]
use std::path::{Component, Path, PathBuf};
#[cfg(feature = "basemind")]
use std::sync::{Arc, Mutex};

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
    /// Optional explicit installed model directory. This is a user-selected
    /// path and is therefore kept separate from the cache path derivation.
    #[serde(default)]
    pub local_path: Option<PathBuf>,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModelType {
    #[default]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Missing,
    Invalid,
    Ready,
    Downloading,
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
#[derive(Clone)]
pub struct ModelManager {
    cache_dir: PathBuf,
    client: reqwest::Client,
    download_lock: Arc<tokio::sync::Mutex<()>>,
    active_downloads: Arc<Mutex<BTreeSet<String>>>,
}

#[cfg(feature = "basemind")]
/// Atomic persistence for the standalone Basemind settings document.
pub struct BasemindSettingsStore {
    path: PathBuf,
}

#[cfg(feature = "basemind")]
const MAX_MODEL_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;

#[cfg(feature = "basemind")]
const MAX_ARCHIVE_ENTRIES: usize = 4096;

#[cfg(feature = "basemind")]
const MAX_ARCHIVE_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[cfg(feature = "basemind")]
impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Self {
            cache_dir,
            client,
            download_lock: Arc::new(tokio::sync::Mutex::new(())),
            active_downloads: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    /// Construct a manager from persisted settings while keeping a stable
    /// caller-provided fallback for older installations without a directory.
    pub fn from_config(config: &BasemindConfig, fallback_cache_dir: PathBuf) -> Self {
        Self::new(config.model_directory.clone().unwrap_or(fallback_cache_dir))
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Prepare a model for a privacy-sensitive call.
    ///
    /// A disabled auto-download setting is never treated as permission to
    /// continue without the model. Missing and invalid required models return
    /// an error, which lets the API redaction path fail closed.
    pub async fn ensure_required_model(
        &self,
        config: &ModelConfig,
        settings: &BasemindConfig,
    ) -> Result<PathBuf> {
        if let Some(override_config) = settings.models.get(&config.name) {
            if config.required && !override_config.enabled {
                anyhow::bail!(
                    "required Basemind model {} is disabled; API redaction is refused",
                    config.name
                );
            }
            if let Some(version) = &override_config.version {
                if version != &config.version {
                    anyhow::bail!(
                        "Basemind model {} is configured for version {}, but the pinned manifest is {}; API redaction is refused until the manifest is updated",
                        config.name,
                        version,
                        config.version
                    );
                }
            }
        }

        if self.status(config) == ModelStatus::Ready {
            return self.model_path(config);
        }
        if !settings.auto_download_models {
            anyhow::bail!(
                "required Basemind model {} is unavailable ({:?}); automatic downloads are disabled, so API redaction is refused",
                config.name,
                self.status(config)
            );
        }
        self.ensure_model(config).await
    }

    pub async fn ensure_model(&self, config: &ModelConfig) -> Result<PathBuf> {
        let model_path = self.model_path(config)?;
        if self.disk_status(config) == ModelStatus::Ready {
            return Ok(model_path);
        }

        let _download_lock = self.download_lock.lock().await;
        if self.disk_status(config) == ModelStatus::Ready {
            return Ok(model_path);
        }

        let model_parent = model_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("model path has no parent directory"))?
            .to_path_buf();
        fs::create_dir_all(&model_parent)?;
        remove_interrupted_downloads(&model_parent, &config.version)?;
        let partial_path = model_parent.join(format!(
            ".{}.partial-{}",
            config.version,
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&partial_path)?;
        let download_key = self.download_key(config)?;
        self.active_downloads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(download_key.clone());

        let result = self.download_and_install(config, &partial_path).await;
        let result = match result {
            Ok(()) => {
                install_downloaded_model(&partial_path, &model_path, &model_parent, &config.version)
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&partial_path);
                Err(error)
            }
        };
        self.active_downloads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&download_key);
        result
    }

    pub fn status(&self, config: &ModelConfig) -> ModelStatus {
        let Ok(download_key) = self.download_key(config) else {
            return ModelStatus::Invalid;
        };
        if self
            .active_downloads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains(&download_key)
        {
            return ModelStatus::Downloading;
        }
        self.disk_status(config)
    }

    fn disk_status(&self, config: &ModelConfig) -> ModelStatus {
        let Ok(model_path) = self.model_path(config) else {
            return ModelStatus::Invalid;
        };
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

    fn model_path(&self, config: &ModelConfig) -> Result<PathBuf> {
        if let Some(local_path) = &config.local_path {
            if local_path.as_os_str().is_empty() {
                anyhow::bail!("invalid local model path: path is empty");
            }
            return Ok(local_path.clone());
        }
        validate_cache_component("model name", &config.name)?;
        validate_cache_component("model version", &config.version)?;
        Ok(self.cache_dir.join(&config.name).join(&config.version))
    }

    fn download_key(&self, config: &ModelConfig) -> Result<String> {
        Ok(self.model_path(config)?.to_string_lossy().into_owned())
    }

    async fn download_and_install(&self, config: &ModelConfig, destination: &Path) -> Result<()> {
        let response = self
            .client
            .get(&config.download_url)
            .send()
            .await?
            .error_for_status()?;
        let max_size = config
            .size_bytes
            .unwrap_or(MAX_MODEL_DOWNLOAD_BYTES)
            .min(MAX_MODEL_DOWNLOAD_BYTES);
        if response
            .content_length()
            .is_some_and(|size| size > max_size)
        {
            anyhow::bail!(
                "model {} exceeds the maximum permitted download size of {} bytes",
                config.name,
                max_size
            );
        }
        let source_path = destination.join("source.bin");
        let mut source = fs::File::create(&source_path)?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            let chunk_size = u64::try_from(chunk.len())?;
            downloaded = downloaded
                .checked_add(chunk_size)
                .ok_or_else(|| anyhow::anyhow!("model download size overflow"))?;
            if downloaded > max_size {
                anyhow::bail!(
                    "model {} exceeds the maximum permitted download size of {} bytes",
                    config.name,
                    max_size
                );
            }
            source.write_all(&chunk)?;
            hasher.update(&chunk);
        }
        source.flush()?;
        drop(source);
        if let Some(expected_size) = config.size_bytes {
            if downloaded != expected_size {
                anyhow::bail!(
                    "model {} has size {}, expected {}",
                    config.name,
                    downloaded,
                    expected_size
                );
            }
        }
        let digest = hex::encode(hasher.finalize());
        if !digest.eq_ignore_ascii_case(&config.sha256) {
            anyhow::bail!(
                "model {} sha256 mismatch: got {}, expected {}",
                config.name,
                digest,
                config.sha256
            );
        }

        match config.archive_format {
            ModelArchiveFormat::None => {
                fs::copy(&source_path, destination.join("model.bin"))?;
            }
            ModelArchiveFormat::Zip => extract_zip(&source_path, destination)?,
        }
        let manifest = ModelManifest {
            name: config.name.clone(),
            version: config.version.clone(),
            sha256: digest,
            size_bytes: downloaded,
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
impl BasemindSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load settings, treating a missing document as the current defaults.
    /// Existing documents are never overwritten merely because a new field
    /// was added; serde supplies defaults in memory and the next explicit save
    /// upgrades the shape.
    pub fn load(&self) -> Result<BasemindConfig> {
        if !self.path.exists() {
            return Ok(BasemindConfig::default());
        }
        let bytes = fs::read(&self.path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Persist settings through a same-directory replace so a terminated
    /// write leaves either the previous document or a complete new document.
    pub fn save(&self, config: &BasemindConfig) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self
            .path
            .with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(config)?;
        if let Err(error) = fs::write(&temporary, bytes) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        Ok(())
    }
}

#[cfg(feature = "basemind")]
fn install_downloaded_model(
    partial_path: &Path,
    model_path: &Path,
    model_parent: &Path,
    version: &str,
) -> Result<PathBuf> {
    let quarantine = model_parent.join(format!(".{}.invalid-{}", version, uuid::Uuid::new_v4()));
    let had_existing = model_path.exists();
    if had_existing {
        fs::rename(model_path, &quarantine)?;
    }
    if let Err(error) = fs::rename(partial_path, model_path) {
        if had_existing {
            let _ = fs::rename(&quarantine, model_path);
        }
        return Err(error.into());
    }
    if had_existing {
        let _ = fs::remove_dir_all(quarantine);
    }
    Ok(model_path.to_path_buf())
}

#[cfg(feature = "basemind")]
fn remove_interrupted_downloads(parent: &Path, version: &str) -> Result<()> {
    let prefix = format!(".{version}.partial-");
    if !parent.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(&prefix) {
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "basemind")]
fn validate_cache_component(label: &str, value: &str) -> Result<()> {
    let mut components = Path::new(value).components();
    if value.is_empty()
        || value.contains(['/', '\\'])
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        anyhow::bail!("invalid {label}: must be one normal path component")
    }
    Ok(())
}

#[cfg(all(feature = "basemind", test))]
fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(feature = "basemind")]
fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(feature = "basemind")]
fn extract_zip(source_path: &Path, destination: &Path) -> Result<()> {
    let source = fs::File::open(source_path)?;
    let mut archive = zip::ZipArchive::new(source)?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        anyhow::bail!(
            "model archive contains {} entries, exceeding the limit of {}",
            archive.len(),
            MAX_ARCHIVE_ENTRIES
        );
    }
    let mut uncompressed_bytes = 0_u64;
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
        let entry_size = entry.size();
        uncompressed_bytes = uncompressed_bytes
            .checked_add(entry_size)
            .ok_or_else(|| anyhow::anyhow!("model archive size overflow"))?;
        if uncompressed_bytes > MAX_ARCHIVE_UNCOMPRESSED_BYTES {
            anyhow::bail!(
                "model archive exceeds the maximum uncompressed size of {} bytes",
                MAX_ARCHIVE_UNCOMPRESSED_BYTES
            );
        }
        let mut output_file = fs::File::create(output)?;
        std::io::copy(&mut entry, &mut output_file)?;
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
            local_path: None,
        };
        assert_eq!(manager.status(&config), ModelStatus::Missing);
        let path = directory.path().join("ner").join("1");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("manifest.json"), "{}").unwrap();
        assert_eq!(manager.status(&config), ModelStatus::Invalid);
    }

    #[test]
    fn a_verified_cache_is_ready() {
        let directory = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(directory.path().to_path_buf());
        let payload = b"verified model";
        let config = ModelConfig {
            name: "ner".into(),
            version: "1".into(),
            download_url: "https://example.invalid/ner.bin".into(),
            sha256: sha256_bytes(payload),
            model_type: ModelType::NerGliner2,
            required: true,
            size_bytes: Some(payload.len() as u64),
            archive_format: ModelArchiveFormat::None,
            local_path: None,
        };
        let path = directory.path().join("ner").join("1");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("source.bin"), payload).unwrap();
        fs::write(
            path.join("manifest.json"),
            serde_json::to_vec(&ModelManifest {
                name: config.name.clone(),
                version: config.version.clone(),
                sha256: config.sha256.clone(),
                size_bytes: payload.len() as u64,
                archive_format: ModelArchiveFormat::None,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(manager.status(&config), ModelStatus::Ready);
    }

    #[tokio::test]
    async fn interrupted_partial_download_is_ignored_and_recovered() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let directory = tempfile::tempdir().unwrap();
        let payload = b"downloaded after interruption".to_vec();
        let digest = sha256_bytes(&payload);
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let response_payload = payload.clone();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_payload.len()
            );
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(&response_payload).await.unwrap();
        });

        let manager = ModelManager::new(directory.path().to_path_buf());
        let config = ModelConfig {
            name: "ner".into(),
            version: "1".into(),
            download_url: format!("http://{address}/ner.bin"),
            sha256: digest,
            model_type: ModelType::NerGliner2,
            required: true,
            size_bytes: Some(payload.len() as u64),
            archive_format: ModelArchiveFormat::None,
            local_path: None,
        };
        let stale = directory.path().join("ner").join(".1.partial-crashed");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("source.bin"), b"truncated").unwrap();
        assert_eq!(manager.status(&config), ModelStatus::Missing);

        let installed = manager.ensure_model(&config).await.unwrap();
        assert_eq!(installed, directory.path().join("ner").join("1"));
        assert_eq!(manager.status(&config), ModelStatus::Ready);
        assert!(!stale.exists());
    }

    #[test]
    fn persisted_settings_use_legacy_defaults_and_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("basemind.json");
        fs::write(&path, r#"{"enabled":true,"auto_download_models":false}"#).unwrap();
        let store = BasemindSettingsStore::new(path.clone());
        let loaded = store.load().unwrap();
        assert!(loaded.enabled);
        assert!(!loaded.auto_download_models);
        assert!(loaded.pseudonymize_prompts);
        assert!(loaded.models.is_empty());

        store.save(&loaded).unwrap();
        assert_eq!(BasemindSettingsStore::new(path).load().unwrap(), loaded);
    }

    #[tokio::test]
    async fn disabled_auto_download_refuses_required_model() {
        let directory = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(directory.path().to_path_buf());
        let config = ModelConfig {
            name: "ner".into(),
            version: "1".into(),
            download_url: "https://example.invalid/ner.bin".into(),
            sha256: "00".repeat(32),
            model_type: ModelType::NerGliner2,
            required: true,
            size_bytes: None,
            archive_format: ModelArchiveFormat::None,
            local_path: None,
        };
        let settings = BasemindConfig {
            auto_download_models: false,
            ..BasemindConfig::default()
        };
        let error = manager
            .ensure_required_model(&config, &settings)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("API redaction is refused"));
    }

    #[test]
    fn sha256_helper_matches_known_payload() {
        let mut hasher = Sha256::new();
        hasher.update(b"model");
        assert_eq!(sha256_bytes(b"model"), hex::encode(hasher.finalize()));
    }

    #[test]
    fn cache_components_cannot_escape_the_model_cache() {
        for value in ["", ".", "..", "../outside", "/tmp/model", "model/version"] {
            assert!(
                validate_cache_component("model name", value).is_err(),
                "{value}"
            );
        }
        assert!(validate_cache_component("model name", "ner-gliner2").is_ok());
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
