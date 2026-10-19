//! Document conversion — single convergence point for all file reads.
//!
//! Routes through Basemind's xberg extractor + redaction, returns redacted markdown
//! plus rehydration map for UI display.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use std::collections::BTreeMap;
#[cfg(feature = "basemind")]
use std::path::Path;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct RedactedDocument {
    pub markdown: String,
    pub rehydration_map: BTreeMap<String, String>, // token -> original value
}

#[cfg(feature = "basemind")]
pub async fn basemind_extract_and_redact(
    _client: &crate::ipc::BasemindClient,
    _path: &Path,
) -> Result<RedactedDocument> {
    // TODO: Call Basemind MCP `redact_text` with file_path
    // Parse response: redacted_text, rehydration_map
    todo!("Implement basemind_extract_and_redact via MCP redact_text tool")
}
