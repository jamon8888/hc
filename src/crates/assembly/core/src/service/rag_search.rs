//! The PII-safe workspace search port, as the product sees it.
//!
//! `openbitfun-core` never talks to the basemind sidecar: it asks the
//! injected `RagSearchPort` for hits and gets back chunks whose personal
//! values are opaque session tokens (D11, SPEC §4). The concrete pipeline —
//! query resolution through the session vault, the sidecar search, and the
//! re-tokenization of every hit — lives in
//! `openbitfun-basemind-integration` and is built by the host at the
//! composition root.
//!
//! The port is published through a process global because the tool context
//! is assembled deep inside the runtime, well below the host: the host
//! registers the port once at startup and every tool context picks it up.

#[cfg(feature = "tools-rag")]
use openbitfun_runtime_ports::{
    DocumentRedactionPort, PromptPseudonymizationPort, RagSearchPort,
};
#[cfg(feature = "tools-rag")]
use std::sync::{OnceLock, RwLock};

#[cfg(feature = "tools-rag")]
static GLOBAL_RAG_SEARCH_PORT: OnceLock<RwLock<Option<std::sync::Arc<dyn RagSearchPort>>>> = OnceLock::new();

#[cfg(feature = "tools-rag")]
static GLOBAL_DOCUMENT_REDACTION_PORT: OnceLock<
    RwLock<Option<std::sync::Arc<dyn DocumentRedactionPort>>>,
> = OnceLock::new();

#[cfg(feature = "tools-rag")]
static GLOBAL_PROMPT_PSEUDONYMIZATION_PORT: OnceLock<
    RwLock<Option<std::sync::Arc<dyn PromptPseudonymizationPort>>>,
> = OnceLock::new();

#[cfg(feature = "tools-rag")]
fn rag_search_port_slot() -> &'static RwLock<Option<std::sync::Arc<dyn RagSearchPort>>> {
    GLOBAL_RAG_SEARCH_PORT.get_or_init(|| RwLock::new(None))
}

/// Publish the workspace search port.
///
/// The host calls this at startup with the port it built. Passing `None`
/// clears it, which is how a host without the sidecar disables the tool.
#[cfg(feature = "tools-rag")]
pub fn set_global_rag_search_port(port: Option<std::sync::Arc<dyn RagSearchPort>>) {
    let mut slot = rag_search_port_slot()
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *slot = port;
}

/// The workspace search port, when the host published one.
///
/// The tool reports its absence as an explicit error rather than an empty
/// result: a search that silently finds nothing is indistinguishable from
/// a workspace that has nothing to find.
#[cfg(feature = "tools-rag")]
pub fn global_rag_search_port() -> Option<std::sync::Arc<dyn RagSearchPort>> {
    rag_search_port_slot()
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// Publish the document redaction port.
///
/// The host calls this at startup with the port it built. Passing `None`
/// clears it, which is how a host without the sidecar keeps the plain
/// document conversion.
#[cfg(feature = "tools-rag")]
pub fn set_global_document_redaction_port(port: Option<std::sync::Arc<dyn DocumentRedactionPort>>) {
    let mut slot = GLOBAL_DOCUMENT_REDACTION_PORT
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *slot = port;
}

/// The document redaction port, when the host published one.
#[cfg(feature = "tools-rag")]
pub fn global_document_redaction_port() -> Option<std::sync::Arc<dyn DocumentRedactionPort>> {
    GLOBAL_DOCUMENT_REDACTION_PORT
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// Publish the prompt pseudonymization port.
///
/// The host calls this at startup with the port it built. Passing `None`
/// clears it, which is how a host without the sidecar refuses to send a
/// prompt at all.
#[cfg(feature = "tools-rag")]
pub fn set_global_prompt_pseudonymization_port(
    port: Option<std::sync::Arc<dyn PromptPseudonymizationPort>>,
) {
    let mut slot = GLOBAL_PROMPT_PSEUDONYMIZATION_PORT
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *slot = port;
}

/// The prompt pseudonymization port, when the host published one.
#[cfg(feature = "tools-rag")]
pub fn global_prompt_pseudonymization_port(
) -> Option<std::sync::Arc<dyn PromptPseudonymizationPort>> {
    GLOBAL_PROMPT_PSEUDONYMIZATION_PORT
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}
