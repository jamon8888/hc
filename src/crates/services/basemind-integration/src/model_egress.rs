//! Basemind-backed model egress protection.
//!
//! The policy runs on the complete provider-bound message list. It deliberately
//! keeps local model traffic untouched and refuses an unknown route rather than
//! guessing whether a custom endpoint is private.

use alex_core_types::{Message, ToolCall};
use alex_runtime_ports::{
    ModelEgressContext, ModelEgressError, ModelEgressErrorKind, ModelEgressPolicy,
    ModelPrivacyRoute, PromptPseudonymizationPort,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::document_convert::BasemindDocumentRedactor;
use crate::identity_registry::IdentityRegistry;
use crate::ipc::{BasemindClient, RedactionPolicy};
use crate::prompt_pseudonym::{BasemindPromptPseudonymizer, PromptPseudonymizer};
use crate::rag_search::{RagSearchPortAdapter, RagSearchTool};
use crate::session_vault::SessionVault;

/// Host-composition bundle shared by Desktop, CLI, server and detached hosts.
///
/// The host still decides where the data lives and which sidecar/model paths
/// are trusted; this crate only assembles the product-owned registry, vault,
/// pseudonymizer and model egress policy consistently.
pub struct BasemindPrivacyBundle {
    pub pseudonymizer: Arc<BasemindPromptPseudonymizer>,
    pub model_egress: Arc<BasemindModelEgressPolicy>,
    pub rag_search: Arc<RagSearchPortAdapter>,
    pub document_redaction: Arc<BasemindDocumentRedactor>,
}

/// Derive a stable, non-reversible storage directory for one workspace.
///
/// The registry and session vault contain encrypted values, but their scope is
/// still a security boundary: a session from one open folder must not share
/// aliases or recovery material with another folder. Hosts that own a
/// workspace should use this helper before calling [`build_privacy_bundle`].
pub fn workspace_data_dir(base_dir: PathBuf, workspace_root: &Path) -> PathBuf {
    let workspace_hash = IdentityRegistry::compute_value_hash(&workspace_root.to_string_lossy());
    base_dir.join("workspaces").join(workspace_hash)
}

pub fn build_privacy_bundle(
    data_dir: PathBuf,
    daemon_binary: Option<PathBuf>,
    ner_model_dir: PathBuf,
    workspace_root: Option<&Path>,
) -> Result<BasemindPrivacyBundle> {
    let workspace_root = workspace_root
        .ok_or_else(|| anyhow::anyhow!("Basemind privacy bundle requires an active workspace"))?;
    let registry_dir = data_dir.join("registry");
    let vault_dir = data_dir.join("vault");
    std::fs::create_dir_all(&registry_dir)?;
    std::fs::create_dir_all(&vault_dir)?;

    let client = BasemindClient::lazy(daemon_binary.clone())
        .with_redaction_policy(RedactionPolicy::bundled(ner_model_dir.clone()));
    // Prompt egress and RAG must share the same in-memory stores. Separate
    // instances would load independent registry snapshots and could overwrite
    // aliases or session maps after concurrent turns, breaking continuity.
    let registry = Arc::new(std::sync::Mutex::new(IdentityRegistry::new(registry_dir)?));
    let vault = Arc::new(std::sync::RwLock::new(SessionVault::new(vault_dir)?));
    let pseudonymizer = Arc::new(BasemindPromptPseudonymizer::new(PromptPseudonymizer::new(
        client,
        registry.clone(),
        vault.clone(),
    )));

    let search_client = BasemindClient::lazy(daemon_binary.clone())
        .with_redaction_policy(RedactionPolicy::bundled(ner_model_dir.clone()));
    let rag_search = Arc::new(RagSearchPortAdapter::new(
        RagSearchTool::new(search_client.clone(), registry, vault),
        workspace_root,
    ));
    let document_redaction = Arc::new(BasemindDocumentRedactor::new(
        BasemindClient::lazy(daemon_binary)
            .with_redaction_policy(RedactionPolicy::bundled(ner_model_dir)),
        pseudonymizer.clone(),
        workspace_root,
    ));

    Ok(BasemindPrivacyBundle {
        model_egress: Arc::new(BasemindModelEgressPolicy::scoped(
            pseudonymizer.clone(),
            Some(workspace_root),
        )),
        pseudonymizer,
        rag_search,
        document_redaction,
    })
}

pub struct BasemindModelEgressPolicy {
    pseudonymizer: Arc<dyn PromptPseudonymizationPort>,
    workspace_root: Option<String>,
}

impl BasemindModelEgressPolicy {
    pub fn new(pseudonymizer: Arc<dyn PromptPseudonymizationPort>) -> Self {
        Self {
            pseudonymizer,
            workspace_root: None,
        }
    }

    pub fn scoped(
        pseudonymizer: Arc<dyn PromptPseudonymizationPort>,
        workspace_root: Option<&Path>,
    ) -> Self {
        Self {
            pseudonymizer,
            workspace_root: workspace_root.map(|path| path.to_string_lossy().into_owned()),
        }
    }

    fn ensure_workspace_scope(&self, context: &ModelEgressContext) -> Result<(), ModelEgressError> {
        let Some(expected_root) = self.workspace_root.as_deref() else {
            return Ok(());
        };
        if context.workspace_root.as_deref() == Some(expected_root) {
            return Ok(());
        }
        Err(ModelEgressError::new(
            ModelEgressErrorKind::MissingContext,
            "model egress refused: request workspace does not match the host privacy scope",
        ))
    }

    async fn pseudonymize_text(
        &self,
        text: String,
        context: &ModelEgressContext,
    ) -> Result<String, ModelEgressError> {
        let session_id = context.session_id.as_deref().ok_or_else(|| {
            ModelEgressError::new(
                ModelEgressErrorKind::MissingContext,
                "model egress refused: API model request has no session id",
            )
        })?;
        let workspace_root = context.workspace_root.as_deref().ok_or_else(|| {
            ModelEgressError::new(
                ModelEgressErrorKind::MissingContext,
                "model egress refused: API model request has no workspace root",
            )
        })?;
        self.pseudonymizer
            .pseudonymize(&text, session_id, workspace_root)
            .await
            .map(|prompt| prompt.text)
            .map_err(|error| {
                ModelEgressError::new(
                    ModelEgressErrorKind::Failed,
                    format!("model egress pseudonymization failed: {error}"),
                )
            })
    }

    async fn pseudonymize_tool_call(
        &self,
        tool_call: &mut ToolCall,
        context: &ModelEgressContext,
    ) -> Result<(), ModelEgressError> {
        let raw_arguments = tool_call.serialized_arguments();
        let pseudonymized = self.pseudonymize_text(raw_arguments, context).await?;
        let arguments = serde_json::from_str(&pseudonymized).map_err(|error| {
            ModelEgressError::new(
                ModelEgressErrorKind::Failed,
                format!("model egress produced invalid tool arguments: {error}"),
            )
        })?;
        tool_call.arguments = arguments;
        tool_call.raw_arguments = Some(pseudonymized);
        Ok(())
    }

    fn contains_image_payload(content: &str) -> bool {
        if content.contains("data:image/") {
            return true;
        }

        fn visit(value: &Value) -> bool {
            match value {
                Value::Array(values) => values.iter().any(visit),
                Value::Object(fields) => fields.iter().any(|(key, value)| {
                    matches!(key.as_str(), "image" | "image_url" | "inline_data") || visit(value)
                }),
                _ => false,
            }
        }

        serde_json::from_str::<Value>(content)
            .map(|value| visit(&value))
            .unwrap_or(false)
    }
}

#[async_trait]
impl ModelEgressPolicy for BasemindModelEgressPolicy {
    async fn protect_messages(
        &self,
        mut messages: Vec<Message>,
        context: ModelEgressContext,
    ) -> Result<Vec<Message>, ModelEgressError> {
        match context.route {
            ModelPrivacyRoute::Local => return Ok(messages),
            ModelPrivacyRoute::Unknown => {
                return Err(ModelEgressError::new(
                    ModelEgressErrorKind::Failed,
                    "model egress refused: model route is unknown",
                ));
            }
            ModelPrivacyRoute::Api => {}
        }
        self.ensure_workspace_scope(&context)?;

        for message in &mut messages {
            if message
                .tool_image_attachments
                .as_ref()
                .is_some_and(|attachments| !attachments.is_empty())
            {
                return Err(ModelEgressError::new(
                    ModelEgressErrorKind::UnsupportedPayload,
                    "model egress refused: image attachments require OCR before an API model call",
                ));
            }
            if let Some(content) = message.content.take() {
                if Self::contains_image_payload(&content) {
                    return Err(ModelEgressError::new(
                        ModelEgressErrorKind::UnsupportedPayload,
                        "model egress refused: image content requires OCR before an API model call",
                    ));
                }
                message.content = Some(self.pseudonymize_text(content, &context).await?);
            }
            if let Some(reasoning) = message.reasoning_content.take() {
                message.reasoning_content =
                    Some(self.pseudonymize_text(reasoning, &context).await?);
            }
            if let Some(tool_calls) = message.tool_calls.as_mut() {
                for tool_call in tool_calls {
                    self.pseudonymize_tool_call(tool_call, &context).await?;
                }
            }
        }

        Ok(messages)
    }

    async fn rehydrate_text(
        &self,
        text: String,
        context: ModelEgressContext,
    ) -> Result<String, ModelEgressError> {
        if context.route == ModelPrivacyRoute::Local {
            return Ok(text);
        }
        if context.route == ModelPrivacyRoute::Unknown {
            return Err(ModelEgressError::new(
                ModelEgressErrorKind::Failed,
                "model response rehydration refused: model route is unknown",
            ));
        }
        self.ensure_workspace_scope(&context)?;
        let session_id = context.session_id.as_deref().ok_or_else(|| {
            ModelEgressError::new(
                ModelEgressErrorKind::MissingContext,
                "model response rehydration refused: API model response has no session id",
            )
        })?;
        self.pseudonymizer
            .rehydrate(&text, session_id)
            .await
            .map_err(|error| {
                ModelEgressError::new(
                    ModelEgressErrorKind::Failed,
                    format!("model response rehydration failed: {error}"),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alex_runtime_ports::{
        ModelEgressContext, ModelPrivacyRoute, PromptPseudonymizationError, PseudonymizedPrompt,
    };

    struct StubPseudonymizer;

    #[async_trait]
    impl PromptPseudonymizationPort for StubPseudonymizer {
        async fn pseudonymize(
            &self,
            prompt: &str,
            session_id: &str,
            _workspace_root: &str,
        ) -> Result<PseudonymizedPrompt, PromptPseudonymizationError> {
            Ok(PseudonymizedPrompt {
                text: prompt.replace("Jean", "[PERSON_test]"),
                rehydration_map: Default::default(),
                session_id: session_id.to_string(),
            })
        }
    }

    fn context(route: ModelPrivacyRoute) -> ModelEgressContext {
        ModelEgressContext {
            route,
            session_id: Some("session-1".into()),
            workspace_root: Some("/workspace".into()),
        }
    }

    #[tokio::test]
    async fn local_models_receive_original_messages() {
        let policy = BasemindModelEgressPolicy::new(Arc::new(StubPseudonymizer));
        let result = policy
            .protect_messages(
                vec![Message::user("Jean".into())],
                context(ModelPrivacyRoute::Local),
            )
            .await
            .unwrap();

        assert_eq!(result[0].content.as_deref(), Some("Jean"));
    }

    #[tokio::test]
    async fn api_models_are_pseudonymized() {
        let policy = BasemindModelEgressPolicy::new(Arc::new(StubPseudonymizer));
        let result = policy
            .protect_messages(
                vec![Message::user("Jean".into())],
                context(ModelPrivacyRoute::Api),
            )
            .await
            .unwrap();

        assert_eq!(result[0].content.as_deref(), Some("[PERSON_test]"));
    }

    #[tokio::test]
    async fn unknown_routes_fail_closed() {
        let policy = BasemindModelEgressPolicy::new(Arc::new(StubPseudonymizer));
        let error = policy
            .protect_messages(
                vec![Message::user("Jean".into())],
                context(ModelPrivacyRoute::Unknown),
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, ModelEgressErrorKind::Failed);
    }

    #[tokio::test]
    async fn api_image_payloads_are_refused_before_pseudonymization() {
        let policy = BasemindModelEgressPolicy::new(Arc::new(StubPseudonymizer));
        let error = policy
            .protect_messages(
                vec![Message::user(
                    r#"[{"type":"image_url","image_url":{"url":"data:image/png;base64,AA=="}}]"#
                        .into(),
                )],
                context(ModelPrivacyRoute::Api),
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, ModelEgressErrorKind::UnsupportedPayload);
    }

    #[tokio::test]
    async fn local_image_payloads_are_left_untouched() {
        let policy = BasemindModelEgressPolicy::new(Arc::new(StubPseudonymizer));
        let content = r#"[{"type":"image_url","image_url":{"url":"data:image/png;base64,AA=="}}]"#;
        let result = policy
            .protect_messages(
                vec![Message::user(content.into())],
                context(ModelPrivacyRoute::Local),
            )
            .await
            .unwrap();

        assert_eq!(result[0].content.as_deref(), Some(content));
    }
}
