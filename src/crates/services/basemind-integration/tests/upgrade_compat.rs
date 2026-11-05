//! Upgrade compatibility canaries for the Basemind service boundary.
//!
//! These tests intentionally deserialize wire shapes produced before workspace
//! identity and the newer optional model fields were introduced. They are
//! contract tests only: no production migration or fallback behavior is added
//! here.

#![cfg(feature = "basemind")]

use alex_basemind_integration::models::{ModelConfig, ModelType};
use alex_basemind_integration::session_vault::{SessionPayload, SessionVault};
use alex_basemind_integration::vault::{vault_decrypt, vault_encrypt};
use serde_json::json;
use std::collections::BTreeMap;
use std::future::Future;

const TEST_PASSPHRASE: &str = "upgrade-compat-test-passphrase";

#[test]
fn legacy_basemind_model_config_deserialization() {
    // The aggregate BasemindConfig from issue 14 is not part of the current
    // production schema yet. ModelConfig is the concrete persisted Basemind
    // configuration type today; these fields were added after the original
    // model manifest/config shape and must keep their defaults.
    let old_json = r#"{
        "name": "ner-gliner2",
        "version": "0.1.0",
        "download_url": "https://example.invalid/ner.zip",
        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "model_type": "NerGliner2"
    }"#;

    let config: ModelConfig = serde_json::from_str(old_json).expect("legacy model config");

    assert_eq!(config.name, "ner-gliner2");
    assert!(matches!(config.model_type, ModelType::NerGliner2));
    assert!(!config.required);
    assert_eq!(config.size_bytes, None);
    assert_eq!(config.archive_format, Default::default());
}

#[test]
fn legacy_session_payload_roundtrips_through_a_reopened_vault() {
    // This is the pre-versioned session shape: the encrypted file contains
    // only the token map and the session salt.
    let legacy_json = r#"{
        "map": { "[PERSON_1]": "Jean Dupont" },
        "salt": "00112233445566778899aabbccddeeff"
    }"#;
    let payload: SessionPayload =
        serde_json::from_str(legacy_json).expect("legacy session payload");
    let temp = tempfile::tempdir().expect("temporary vault directory");
    let vault =
        SessionVault::with_passphrase(temp.path().to_path_buf(), TEST_PASSPHRASE.to_string());

    vault
        .store_session("session:test:prompt_rehydration", payload.clone())
        .expect("store session");

    let reopened =
        SessionVault::with_passphrase(temp.path().to_path_buf(), TEST_PASSPHRASE.to_string());
    let loaded = reopened
        .load_session("session:test:prompt_rehydration")
        .expect("load session");
    assert_eq!(loaded.map, payload.map);
    assert_eq!(loaded.salt, payload.salt);
}

#[test]
fn current_vault_blob_is_readable_after_restart() {
    let payload = SessionPayload {
        map: BTreeMap::from([("[PERSON_1]".to_string(), "Jean Dupont".to_string())]),
        salt: "00112233445566778899aabbccddeeff".to_string(),
    };
    let temp = tempfile::tempdir().expect("temporary vault directory");
    let session_id = "session:legacy:vault";
    let vault_path = temp.path().join(format!("{session_id}.vault"));

    // The legacy-salt fallback is covered by the unit fixture in vault.rs;
    // this test keeps the persisted payload/restart contract explicit.
    let encrypted = vault_encrypt(&payload, TEST_PASSPHRASE).expect("encrypt legacy vault");
    std::fs::write(&vault_path, encrypted).expect("write legacy vault");

    let reopened =
        SessionVault::with_passphrase(temp.path().to_path_buf(), TEST_PASSPHRASE.to_string());
    let loaded = reopened
        .load_session(session_id)
        .expect("load legacy vault");
    assert_eq!(loaded.map, payload.map);
    assert_eq!(loaded.salt, payload.salt);

    // Also keep the generic vault round-trip explicit for future format
    // changes: the current reader must still decode a legacy serializable
    // value without a production-side migration step.
    let generic = json!({ "map": { "[EMAIL_1]": "jean@example.com" }, "salt": "salt" });
    let encoded = vault_encrypt(&generic, TEST_PASSPHRASE).expect("encrypt generic payload");
    let decoded: serde_json::Value =
        vault_decrypt(&encoded, TEST_PASSPHRASE).expect("decrypt generic payload");
    assert_eq!(decoded, generic);
}

#[test]
fn upstream_conversion_signature_unchanged() {
    // A type-checked function item is enough to make an upstream argument or
    // output-shape change fail this test at compile time. The converter exists
    // behind the document-read feature in the current tool-runtime crate.
    fn assert_signature<F, Fut>(_: F)
    where
        F: Fn(Vec<u8>, String) -> Fut,
        Fut: Future<
            Output = Result<
                tool_runtime::fs::document::ConvertedDocument,
                tool_runtime::fs::document::DocumentConversionError,
            >,
        >,
    {
    }

    assert_signature(tool_runtime::fs::document::convert_document_to_markdown);
}
