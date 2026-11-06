//! Integration-level closure tests for the public Basemind egress policy.

#![cfg(feature = "basemind")]

use alex_basemind_integration::BasemindModelEgressPolicy;
use alex_core_types::Message;
use alex_runtime_ports::{
    ModelEgressContext, ModelEgressErrorKind, ModelEgressPolicy, ModelPrivacyRoute,
    PromptPseudonymizationError, PromptPseudonymizationPort, PseudonymizedPrompt,
};
use async_trait::async_trait;
use std::sync::Arc;

struct FakePseudonymizer;

#[async_trait]
impl PromptPseudonymizationPort for FakePseudonymizer {
    async fn pseudonymize(
        &self,
        prompt: &str,
        session_id: &str,
        _workspace_root: &str,
    ) -> Result<PseudonymizedPrompt, PromptPseudonymizationError> {
        Ok(PseudonymizedPrompt {
            text: prompt.replace("Jean Dupont", "[PERSON_1]"),
            rehydration_map: Default::default(),
            session_id: session_id.to_string(),
        })
    }
}

fn context(route: ModelPrivacyRoute) -> ModelEgressContext {
    ModelEgressContext {
        route,
        session_id: Some("session:closure".to_string()),
        workspace_root: Some("/workspace/case".to_string()),
    }
}

fn policy() -> BasemindModelEgressPolicy {
    BasemindModelEgressPolicy::scoped(
        Arc::new(FakePseudonymizer),
        Some(std::path::Path::new("/workspace/case")),
    )
}

#[tokio::test]
async fn api_payload_is_pseudonymized_but_local_payload_is_untouched() {
    let api = policy()
        .protect_messages(
            vec![Message::user("Contact Jean Dupont.".to_string())],
            context(ModelPrivacyRoute::Api),
        )
        .await
        .expect("API route should be protected");
    assert_eq!(api[0].content.as_deref(), Some("Contact [PERSON_1]."));

    let local = policy()
        .protect_messages(
            vec![Message::user("Contact Jean Dupont.".to_string())],
            context(ModelPrivacyRoute::Local),
        )
        .await
        .expect("local route should remain available");
    assert_eq!(local[0].content.as_deref(), Some("Contact Jean Dupont."));
}

#[tokio::test]
async fn unknown_route_fails_closed_before_pseudonymization() {
    let error = policy()
        .protect_messages(
            vec![Message::user("Contact Jean Dupont.".to_string())],
            context(ModelPrivacyRoute::Unknown),
        )
        .await
        .expect_err("unknown route must not reach a provider");

    assert_eq!(error.kind, ModelEgressErrorKind::Failed);
}

#[tokio::test]
async fn api_context_without_session_or_workspace_fails_closed() {
    let mut missing_session = context(ModelPrivacyRoute::Api);
    missing_session.session_id = None;
    let error = policy()
        .protect_messages(
            vec![Message::user("Jean Dupont".to_string())],
            missing_session,
        )
        .await
        .expect_err("API route without a session must refuse");
    assert_eq!(error.kind, ModelEgressErrorKind::MissingContext);

    let mut missing_workspace = context(ModelPrivacyRoute::Api);
    missing_workspace.workspace_root = None;
    let error = policy()
        .protect_messages(
            vec![Message::user("Jean Dupont".to_string())],
            missing_workspace,
        )
        .await
        .expect_err("API route without a workspace must refuse");
    assert_eq!(error.kind, ModelEgressErrorKind::MissingContext);
}

#[tokio::test]
async fn unsupported_api_image_payload_fails_closed() {
    let error = policy()
        .protect_messages(
            vec![Message::user(
                r#"[{"type":"image_url","image_url":{"url":"data:image/png;base64,AA=="}}]"#
                    .to_string(),
            )],
            context(ModelPrivacyRoute::Api),
        )
        .await
        .expect_err("unsupported API images must not be sent");

    assert_eq!(error.kind, ModelEgressErrorKind::UnsupportedPayload);
}
