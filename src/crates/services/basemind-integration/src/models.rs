//! Model specifications and management for Basemind.

#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use std::path::PathBuf;

/// Model profile determining which capabilities are enabled.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProfile {
    /// Full pipeline: NER, embeddings, OCR, summarization
    Full,
    /// Code-only: embeddings only, no NER/OCR
    CodeOnly,
    /// None: no models loaded
    None,
}

impl Default for ModelProfile {
    fn default() -> Self {
        ModelProfile::Full
    }
}

/// Specification for a Basemind model.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub name: String,           // "pii-multilingual", "embedding-minilm", "ocr-multilingual"
    pub version: String,
    pub download_url: String,   // HF Hub primary, GitHub Releases fallback
    pub sha256: String,
    pub required: bool,         // false = optional (OCR)
    pub size_bytes: u64,
}

/// Predefined model specifications.
#[cfg(feature = "basemind")]
impl ModelSpec {
    /// PII detection (NER) for 50+ languages.
    pub fn pii_multilingual() -> Self {
        Self {
            name: "pii-multilingual".to_string(),
            version: "1.0.0".to_string(),
            download_url: "https://huggingface.co/jamon8888/basemind-pii-multilingual/resolve/main/model.onnx".to_string(),
            sha256: "TODO_SHA256".to_string(),
            required: true,
            size_bytes: 150_000_000, // ~150 MB
        }
    }

    /// Embedding model for RAG vector index (MiniLM).
    pub fn embedding_minilm() -> Self {
        Self {
            name: "embedding-minilm".to_string(),
            version: "1.0.0".to_string(),
            download_url: "https://huggingface.co/jamon8888/basemind-embedding-minilm/resolve/main/model.onnx".to_string(),
            sha256: "TODO_SHA256".to_string(),
            required: true,
            size_bytes: 90_000_000, // ~90 MB
        }
    }

    /// OCR model for image text extraction.
    pub fn ocr_multilingual() -> Self {
        Self {
            name: "ocr-multilingual".to_string(),
            version: "1.0.0".to_string(),
            download_url: "https://huggingface.co/jamon8888/basemind-ocr-multilingual/resolve/main/model.onnx".to_string(),
            sha256: "TODO_SHA256".to_string(),
            required: false,
            size_bytes: 200_000_000, // ~200 MB
        }
    }

    /// All required models for full profile.
    pub fn required_models() -> Vec<Self> {
        vec![Self::pii_multilingual(), Self::embedding_minilm()]
    }

    /// All models for full profile including optional OCR.
    pub fn all_models() -> Vec<Self> {
        vec![Self::pii_multilingual(), Self::embedding_minilm(), Self::ocr_multilingual()]
    }
}

/// Model configuration stored in user settings.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub version: String,
    pub enabled: bool,
    pub local_path: Option<PathBuf>,
}

/// Extract result from Basemind.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub markdown: String,
    pub source_format: String,
    pub entities: Vec<RedactedEntity>,
}

/// PII entity found during extraction.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub occurrence_count: u32,
}

/// RAG search hit.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagHit {
    pub chunk: String,
    pub source_doc: String,
    pub score: f32,
    pub entity_ids: Vec<String>,
}

/// PII finding for prompt pseudonymization.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiFinding {
    pub start: usize,
    pub end: usize,
    pub category: String,
    pub confidence: f32,
}