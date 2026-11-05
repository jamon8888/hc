//! Model egress policy contract.
//!
//! This is the last product-neutral seam before an AI adapter serializes a
//! request for a provider. Hosts inject the concrete privacy policy here;
//! adapters must never infer privacy from a provider name alone.

use alex_core_types::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPrivacyRoute {
    Api,
    Local,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEgressContext {
    pub route: ModelPrivacyRoute,
    pub session_id: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelEgressErrorKind {
    MissingContext,
    UnsupportedPayload,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEgressError {
    pub kind: ModelEgressErrorKind,
    pub message: String,
}

impl ModelEgressError {
    pub fn new(kind: ModelEgressErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for ModelEgressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ModelEgressError {}

/// Protects the complete provider-bound message list before serialization.
#[async_trait]
pub trait ModelEgressPolicy: Send + Sync {
    async fn protect_messages(
        &self,
        messages: Vec<Message>,
        context: ModelEgressContext,
    ) -> Result<Vec<Message>, ModelEgressError>;

    /// Restore session tokens only for an explicitly authorized human-rendering
    /// adapter. Runtime transcript, traces, tool calls, and model context must
    /// retain tokens. The default keeps compatibility with non-reversible
    /// policies.
    async fn rehydrate_text(
        &self,
        text: String,
        _context: ModelEgressContext,
    ) -> Result<String, ModelEgressError> {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelEgressContext, ModelPrivacyRoute};

    #[test]
    fn context_keeps_route_and_workspace_scoped_identity() {
        let context = ModelEgressContext {
            route: ModelPrivacyRoute::Api,
            session_id: Some("session-1".into()),
            workspace_root: Some("/workspace".into()),
        };

        assert_eq!(context.route, ModelPrivacyRoute::Api);
        assert_eq!(context.session_id.as_deref(), Some("session-1"));
    }
}
