//! Document conversion via Basemind — replaces `convert_document_to_markdown` from tool-execution.

#[cfg(feature = "basemind")]
use std::sync::Arc;

#[cfg(feature = "basemind")]
use crate::ipc::BasemindClient;
#[cfg(feature = "basemind")]
use crate::models::{ExtractResult, RedactedEntity};
#[cfg(feature = "basemind")]
use anyhow::Result;

/// Maximum source-document size accepted by the Read tool conversion path.
#[cfg(feature = "basemind")]
pub const MAX_DOCUMENT_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// Document conversion error codes.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocumentConversionError {
    #[error("resource limit: {1}")]
    ResourceLimit(&'static str, String),
    #[error("runtime error: {1}")]
    Runtime(&'static str, String),
}

#[cfg(feature = "basemind")]
impl DocumentConversionError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        match code {
            "resourceLimit" => Self::ResourceLimit(code, message.into()),
            _ => Self::Runtime(code, message.into()),
        }
    }
    
    pub fn code(&self) -> &'static str {
        match self {
            Self::ResourceLimit(c, _) => c,
            Self::Runtime(c, _) => c,
        }
    }
}

/// Infer source format from path hint.
#[cfg(feature = "basemind")]
fn infer_source_format(path_hint: &str) -> &'static str {
    let path = std::path::Path::new(path_hint);
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => "pdf",
        Some("docx") | Some("doc") | Some("docm") => "docx",
        Some("odt") => "odt",
        Some("xlsx") | Some("xls") | Some("xlsm") | Some("xlsb") => "excel",
        Some("ods") => "ods",
        Some("pptx") | Some("ppt") | Some("pptm") | Some("ppsx") => "pptx",
        Some("odp") => "odp",
        Some("rtf") => "rtf",
        Some("epub") => "epub",
        Some("csv") => "csv",
        _ => "unknown",
    }
}

/// Basemind-backed document conversion with PII redaction.
/// Drop-in replacement for `tool_runtime::fs::document::convert_document_to_markdown`.
#[cfg(feature = "basemind")]
pub async fn basemind_extract_and_redact(
    bytes: Vec<u8>,
    path_hint: String,
) -> Result<ExtractResult, DocumentConversionError> {
    if bytes.len() > MAX_DOCUMENT_INPUT_BYTES {
        return Err(DocumentConversionError::new(
            "resourceLimit",
            format!(
                "document is larger than the {} MiB Read limit",
                MAX_DOCUMENT_INPUT_BYTES / (1024 * 1024)
            ),
        ));
    }

    // Acquire semaphore to bound concurrent conversions (same as upstream)
    let permit = document_conversion_semaphore()
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| DocumentConversionError::new("runtime", "conversion worker closed"))?;

    // Get Basemind client (initialized lazily)
    let basemind = basemind_client();

    let result = basemind
        .extract_and_redact(bytes, path_hint.clone())
        .await
        .map_err(|e| DocumentConversionError::new("runtime", e.to_string()))?;

    drop(permit);

    Ok(ExtractResult {
        markdown: result.markdown,
        source_format: infer_source_format(&path_hint).to_string(),
        entities: result.entities,
    })
}

/// Global semaphore for bounding concurrent document conversions (same as upstream).
#[cfg(feature = "basemind")]
fn document_conversion_semaphore() -> &'static Arc<tokio::sync::Semaphore> {
    use std::sync::OnceLock;
    static SEMAPHORE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    SEMAPHORE.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
}

/// Lazily-initialized Basemind client for document conversion.
#[cfg(feature = "basemind")]
fn basemind_client() -> Arc<BasemindClient> {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<Arc<BasemindClient>> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Arc::new(BasemindClient::new_uninitialized())
    }).clone()
}