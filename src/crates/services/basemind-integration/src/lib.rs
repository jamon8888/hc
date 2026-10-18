//! Basemind integration for OpenBitFun
//!
//! Provides document extraction/redaction, RAG search, and prompt pseudonymization
//! via the Basemind engine (jamon8888/basemind fork).

#[cfg(feature = "basemind")]
pub mod workspace_fs;

#[cfg(feature = "basemind")]
pub mod document_convert;

#[cfg(feature = "basemind")]
pub mod prompt_pseudonym;

#[cfg(feature = "basemind")]
pub mod session_vault;

#[cfg(feature = "basemind")]
pub mod local_model_detection;

#[cfg(feature = "basemind")]
pub mod ipc;

#[cfg(feature = "basemind")]
pub mod models;

#[cfg(feature = "basemind")]
pub mod vault;

// Re-exports for convenience
#[cfg(feature = "basemind")]
pub use workspace_fs::BasemindWorkspaceFs;

#[cfg(feature = "basemind")]
pub use document_convert::basemind_extract_and_redact;

#[cfg(feature = "basemind")]
pub use models::{ExtractResult, RedactedEntity};

#[cfg(feature = "basemind")]
pub use prompt_pseudonym::{PromptPseudonymizer, PseudonymError};

#[cfg(feature = "basemind")]
pub use session_vault::{SessionVault, VaultError};

#[cfg(feature = "basemind")]
pub use local_model_detection::is_local_model_base_url;

#[cfg(feature = "basemind")]
pub use ipc::{BasemindClient, BasemindTransport, BasemindError};

#[cfg(feature = "basemind")]
pub use models::{ModelSpec, ModelProfile};

#[cfg(feature = "basemind")]
pub use vault::{vault_encrypt, vault_decrypt};