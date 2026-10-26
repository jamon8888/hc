//! Basemind integration crate.
//!
//! Isolates all Basemind sidecar logic (PII detection, redaction, RAG, identity registry,
//! vault, prompt pseudonymization) from upstream Alex code. Communicates with the
//! Basemind sidecar via MCP over HTTP; the sidecar is a separate process (`basemind comms daemon`).

#![cfg_attr(feature = "basemind", allow(unused_imports))]

#[cfg(feature = "basemind")]
pub mod daemon;
#[cfg(feature = "basemind")]
pub mod document_convert;
#[cfg(feature = "basemind")]
pub mod gateway;
#[cfg(feature = "basemind")]
pub mod identity_registry;
#[cfg(feature = "basemind")]
pub mod ipc;
#[cfg(feature = "basemind")]
pub mod keychain;
#[cfg(feature = "basemind")]
pub mod models;
#[cfg(feature = "basemind")]
pub mod prompt_pseudonym;
#[cfg(feature = "basemind")]
pub mod query_resolution;
#[cfg(feature = "basemind")]
pub mod rag_search;
#[cfg(feature = "basemind")]
pub mod redaction;
#[cfg(feature = "basemind")]
pub mod session_vault;
#[cfg(feature = "basemind")]
pub mod token;
#[cfg(feature = "basemind")]
pub mod vault;
#[cfg(feature = "basemind")]
pub mod workspace_fs;

#[cfg(feature = "basemind")]
pub use self::daemon::{
    daemon_version, verify_version, DaemonEndpoint, BASEMIND_VERSION, COMMS_DIR_ENV,
};
#[cfg(feature = "basemind")]
pub use self::document_convert::BasemindDocumentRedactor;
#[cfg(feature = "basemind")]
pub use self::gateway::{generate_gateway_token, is_protected_workspace, GATEWAY_TOKEN_ENV};
#[cfg(feature = "basemind")]
pub use self::identity_registry::IdentityRegistry;
#[cfg(feature = "basemind")]
pub use self::ipc::{BasemindClient, DocEntity, DocumentSearchHit, RedactionPolicy};
#[cfg(feature = "basemind")]
pub use self::keychain::{get_vault_passphrase, set_vault_passphrase, KeychainError};
#[cfg(feature = "basemind")]
pub use self::models::{ExtractResult, ModelConfig, ModelManager, RedactedEntity};
#[cfg(feature = "basemind")]
pub use self::prompt_pseudonym::{
    BasemindPromptPseudonymizer, PromptPseudonymizer, PseudonymizedPrompt,
};
#[cfg(feature = "basemind")]
pub use self::query_resolution::{QueryResolver, RetokenizedEntity, RetokenizedHit};
#[cfg(feature = "basemind")]
pub use self::rag_search::{RagSearchInput, RagSearchOutput, RagSearchPortAdapter, RagSearchTool};
#[cfg(feature = "basemind")]
pub use self::redaction::{RedactedDocument, RedactionStore};
#[cfg(feature = "basemind")]
pub use self::session_vault::SessionVault;
#[cfg(feature = "basemind")]
pub use self::token::mint_session_token;
#[cfg(feature = "basemind")]
pub use self::vault::{vault_decrypt, vault_encrypt, VaultError};
/// Re-exports for consumers when the `basemind` feature is enabled.
#[cfg(feature = "basemind")]
pub use self::workspace_fs::BasemindWorkspaceFs;
