//! PII-safe prompt pseudonymization port.
//!
//! The only shape `openbitfun-core` ever sees for prompt interception
//! (SPEC §11.4, invariant I9): the prompt the model receives carries
//! opaque session tokens, never the personal values.
//!
//! The concrete pseudonymizer lives in `openbitfun-basemind-integration` and
//! is published by the host at the composition root, the same way the
//! `RagSearchPort` and the `DocumentRedactionPort` are published (D11).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A pseudonymized prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudonymizedPrompt {
    /// The prompt as the model sees it: session tokens, never the values.
    pub text: String,
    /// The session tokens minted for this prompt, mapped to their original
    /// values. The product keeps this; it must not reach the model.
    pub rehydration_map: BTreeMap<String, String>,
    /// The session the tokens belong to.
    pub session_id: String,
}

/// Why pseudonymization failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptPseudonymizationErrorKind {
    /// The request cannot be pseudonymized as asked.
    InvalidRequest,
    /// The sidecar or the session vault could not be reached.
    Unavailable,
    /// The sidecar answered with something the product cannot use.
    InvalidResponse,
    /// The pseudonymization failed.
    Failed,
}

/// A pseudonymization failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPseudonymizationError {
    pub kind: PromptPseudonymizationErrorKind,
    pub message: String,
}

impl PromptPseudonymizationError {
    pub fn new(kind: PromptPseudonymizationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for PromptPseudonymizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PromptPseudonymizationError {}

/// Replaces the personal values in a prompt with opaque session tokens.
#[async_trait]
pub trait PromptPseudonymizationPort: Send + Sync {
    /// Pseudonymize `prompt` for `session_id`, scoped to `workspace_root`.
    async fn pseudonymize(
        &self,
        prompt: &str,
        session_id: &str,
        workspace_root: &str,
    ) -> Result<PseudonymizedPrompt, PromptPseudonymizationError>;
}
