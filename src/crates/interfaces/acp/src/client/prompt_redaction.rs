//! PII-safe prompt interception seam for prompts crossing to external ACP
//! agents (SPEC §11.4, invariant I9, extended to ACP clients).
//!
//! The prompt an external agent receives carries session tokens, never the
//! personal values. This applies to every external agent the product drives
//! over ACP — the OpenCode, Claude Code and Codex clients, and any bundled
//! interpreter that joins through the same client layer.
//!
//! The concrete pseudonymizer belongs to the composition root that also owns
//! the basemind sidecar, so the seam is a small local trait instead of a
//! port dependency: a host that ships PII-safe prompting installs an
//! interceptor at startup through
//! [`set_prompt_interceptor`], and products that do not ship the guarantee
//! stay on the pass-through path — the same semantics the native loop has
//! behind its build-time `basemind` feature.
//!
//! # Public contract note
//!
//! Stable ACP host contract (`contracts/acp` boundary slice).
//!
//! - **Version impact**: Minor. Adding an interceptor method or changing the
//!   trait is a breaking change for implementing hosts and must not land
//!   without a replacement surface first.
//! - **Verification command**: `cargo test -p alex-acp --lib
//!   acp_prompt_interception` and `pnpm run check:core-boundaries` (the seam
//!   must stay free of upward dependencies).
//! - **Removal condition**: removable only once no composition root installs
//!   an interceptor and the external-agent surfaces (desktop, CLI, and the
//!   bundled interpreters over ACP) either ship an equivalent PII seam
//!   elsewhere or drop the PII-safe-prompting promise in their feature docs.
//!   grep-verified absence of `set_prompt_interceptor` installations is the
//!   removal gate.
//!
//! Installed or not, the seam never lets an API route fail open by accident:
//! when an installed interceptor errors, the prompt aborts; a local route is
//! explicitly allowed to pass through because its model is not an external
//! API boundary.

use std::path::Path;
use std::sync::Arc;

use alex_core::util::errors::AlexResult;
use alex_runtime_ports::ModelPrivacyRoute;

/// The seam the composition root fills when it ships PII-safe prompting.
#[async_trait::async_trait]
pub trait AcpPromptInterceptor: Send + Sync {
    /// Rewrite the prompt so it carries session tokens instead of the
    /// personal values.
    ///
    /// Implementations fail closed: an error aborts the prompt, it must
    /// never fall back to the raw text.
    async fn pseudonymize(
        &self,
        prompt: &str,
        workspace_root: &Path,
        alex_session_id: &str,
        route: ModelPrivacyRoute,
    ) -> AlexResult<String>;

    /// Restore session tokens in the external agent's response before it is
    /// shown to the user. Implementations must fail closed on storage errors.
    async fn rehydrate(
        &self,
        text: &str,
        _workspace_root: &Path,
        _alex_session_id: &str,
        _route: ModelPrivacyRoute,
    ) -> AlexResult<String> {
        Ok(text.to_string())
    }
}

/// Host-owned fail-closed interceptor for ACP surfaces that have not yet
/// published a concrete Basemind policy.
pub struct RefusingAcpPromptInterceptor {
    message: String,
}

/// ACP adapter for a host-owned Basemind prompt pseudonymizer.
pub struct PseudonymizingAcpPromptInterceptor {
    port: Arc<dyn alex_runtime_ports::PromptPseudonymizationPort>,
}

impl PseudonymizingAcpPromptInterceptor {
    pub fn new(port: Arc<dyn alex_runtime_ports::PromptPseudonymizationPort>) -> Self {
        Self { port }
    }
}

#[async_trait::async_trait]
impl AcpPromptInterceptor for PseudonymizingAcpPromptInterceptor {
    async fn pseudonymize(
        &self,
        prompt: &str,
        workspace_root: &Path,
        alex_session_id: &str,
        route: ModelPrivacyRoute,
    ) -> AlexResult<String> {
        if route == ModelPrivacyRoute::Local {
            return Ok(prompt.to_string());
        }
        if route == ModelPrivacyRoute::Unknown {
            return Err(alex_core::util::errors::AlexError::service(
                "ACP prompt refused: external model route is unknown",
            ));
        }
        self.port
            .pseudonymize(prompt, alex_session_id, &workspace_root.to_string_lossy())
            .await
            .map(|prompt| prompt.text)
            .map_err(|error| {
                alex_core::util::errors::AlexError::service(format!(
                    "PII pseudonymization failed (Basemind required): {error}"
                ))
            })
    }

    async fn rehydrate(
        &self,
        text: &str,
        _workspace_root: &Path,
        alex_session_id: &str,
        route: ModelPrivacyRoute,
    ) -> AlexResult<String> {
        if route == ModelPrivacyRoute::Local {
            return Ok(text.to_string());
        }
        if route == ModelPrivacyRoute::Unknown {
            return Err(alex_core::util::errors::AlexError::service(
                "ACP response rehydration refused: external model route is unknown",
            ));
        }
        self.port
            .rehydrate(text, alex_session_id)
            .await
            .map_err(|error| {
                alex_core::util::errors::AlexError::service(format!(
                    "ACP response rehydration failed (Basemind required): {error}"
                ))
            })
    }
}

impl RefusingAcpPromptInterceptor {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait::async_trait]
impl AcpPromptInterceptor for RefusingAcpPromptInterceptor {
    async fn pseudonymize(
        &self,
        _prompt: &str,
        _workspace_root: &Path,
        _alex_session_id: &str,
        _route: ModelPrivacyRoute,
    ) -> AlexResult<String> {
        Err(alex_core::util::errors::AlexError::service(
            self.message.clone(),
        ))
    }
}

static PROMPT_INTERCEPTOR: std::sync::OnceLock<
    std::sync::RwLock<Option<Arc<dyn AcpPromptInterceptor>>>,
> = std::sync::OnceLock::new();

/// Install (or remove) the prompt interceptor.
///
/// The host calls this at startup, the same place it publishes its other
/// PII-safe ports. Passing `None` removes an installed interceptor, which is
/// how a host without the sidecar disables the ACP privacy capability.
pub fn set_prompt_interceptor(interceptor: Option<Arc<dyn AcpPromptInterceptor>>) {
    let mut slot = PROMPT_INTERCEPTOR
        .get_or_init(|| std::sync::RwLock::new(None))
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *slot = interceptor;
}

/// The installed interceptor, when a host published one.
fn prompt_interceptor() -> Option<Arc<dyn AcpPromptInterceptor>> {
    PROMPT_INTERCEPTOR
        .get_or_init(|| std::sync::RwLock::new(None))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// Run the installed interceptor over a prompt about to cross to an external
/// agent. External routes must fail closed when no host published the privacy
/// guarantee; raw prompts must never silently cross this boundary.
pub async fn apply_prompt_interceptor(
    prompt: String,
    workspace_root: &Path,
    alex_session_id: &str,
    route: ModelPrivacyRoute,
) -> AlexResult<String> {
    if route == ModelPrivacyRoute::Local {
        return Ok(prompt);
    }
    let interceptor = prompt_interceptor().ok_or_else(|| {
        alex_core::util::errors::AlexError::service(
            "ACP prompt refused: PII pseudonymization is unavailable",
        )
    })?;
    interceptor
        .pseudonymize(&prompt, workspace_root, alex_session_id, route)
        .await
}

pub async fn rehydrate_prompt_interceptor(
    text: String,
    workspace_root: &Path,
    alex_session_id: &str,
    route: ModelPrivacyRoute,
) -> AlexResult<String> {
    match prompt_interceptor() {
        Some(interceptor) => {
            interceptor
                .rehydrate(&text, workspace_root, alex_session_id, route)
                .await
        }
        None if route == ModelPrivacyRoute::Local => Ok(text),
        None => Err(alex_core::util::errors::AlexError::service(
            "ACP response rehydration refused: PII pseudonymization is unavailable",
        )),
    }
}
