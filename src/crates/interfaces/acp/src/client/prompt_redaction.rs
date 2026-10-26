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
//! Installed or not, the seam never lets a host fail open by accident: when
//! an installed interceptor errors, the prompt aborts; only a host without
//! an interceptor passes raw text, and that host never published the PII
//! guarantee in the first place.

use std::path::Path;
use std::sync::Arc;

use alex_core::util::errors::AlexResult;

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
    ) -> AlexResult<String>;
}

static PROMPT_INTERCEPTOR: std::sync::OnceLock<
    std::sync::RwLock<Option<Arc<dyn AcpPromptInterceptor>>>,
> = std::sync::OnceLock::new();

/// Install (or remove) the prompt interceptor.
///
/// The host calls this at startup, the same place it publishes its other
/// PII-safe ports. Passing `None` removes an installed interceptor, which is
/// how a host without the sidecar passes prompts through.
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
/// agent, or return the prompt unchanged when no host published one.
pub async fn apply_prompt_interceptor(
    prompt: String,
    workspace_root: &Path,
    alex_session_id: &str,
) -> AlexResult<String> {
    match prompt_interceptor() {
        Some(interceptor) => {
            interceptor
                .pseudonymize(&prompt, workspace_root, alex_session_id)
                .await
        }
        None => Ok(prompt),
    }
}
