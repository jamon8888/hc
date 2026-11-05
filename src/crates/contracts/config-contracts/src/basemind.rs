//! Persisted Basemind settings.
//!
//! This contract is independent from the Basemind service so hosts can embed
//! the same shape in their configuration without changing the on-disk format.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn default_enabled() -> bool {
    true
}

fn default_auto_download_models() -> bool {
    true
}

fn default_pseudonymize_prompts() -> bool {
    true
}

/// User-facing Basemind settings.
///
/// Missing fields use compatibility defaults. Disabling automatic downloads
/// never disables privacy: the service refuses API redaction when a required
/// model is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BasemindConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_auto_download_models")]
    pub auto_download_models: bool,
    pub model_directory: Option<PathBuf>,
    pub redact_filenames: bool,
    pub models: BTreeMap<String, BasemindModelConfig>,
    pub sidecar_path: Option<PathBuf>,
    #[serde(default = "default_pseudonymize_prompts")]
    pub pseudonymize_prompts: bool,
}

impl Default for BasemindConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            auto_download_models: default_auto_download_models(),
            model_directory: None,
            redact_filenames: false,
            models: BTreeMap::new(),
            sidecar_path: None,
            pseudonymize_prompts: default_pseudonymize_prompts(),
        }
    }
}

/// Persisted per-model overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BasemindModelConfig {
    pub version: Option<String>,
    pub enabled: bool,
    pub local_path: Option<PathBuf>,
}

impl Default for BasemindModelConfig {
    fn default() -> Self {
        Self {
            version: None,
            enabled: true,
            local_path: None,
        }
    }
}
