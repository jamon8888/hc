//! Unit tests for the pure parts of the basemind integration.
//!
//! These cover the pieces that decide what the product sends to the fork and
//! how it reads the answer back: the query-string encoding, the MCP result
//! unwrapping, the portfile parsing, and the token-to-span rewrite.

#![cfg(feature = "basemind")]

use openbitfun_basemind_integration::daemon;
use openbitfun_basemind_integration::ipc::{extract_tool_payload, urlencoding};
use openbitfun_basemind_integration::redaction::{category_of, rewrite_tokens_as_spans};
use std::collections::BTreeMap;

/// A temp directory that is unique to this test process, so concurrent test
/// runs on one machine never delete or overwrite each other's registry and
/// vault files.
fn test_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("openbitfun-basemind-{name}-{}", std::process::id()))
}

#[test]
fn query_values_are_form_urlencoded() {
    // The daemon decodes the query with `form_urlencoded`, so the encoding
    // must be the same one: a path with spaces or non-ASCII characters
    // otherwise changes the query the daemon parses.
    assert_eq!(urlencoding("/tmp/plain"), "%2Ftmp%2Fplain");
    assert_eq!(urlencoding("/tmp/with space"), "%2Ftmp%2Fwith+space");
    assert_eq!(
        urlencoding("/tmp/Users/jamín/文档"),
        "%2Ftmp%2FUsers%2Fjam%C3%ADn%2F%E6%96%87%E6%A1%A3"
    );
    // Characters that carry meaning in a query string must not survive raw.
    assert_eq!(urlencoding("/a?b&c=d"), "%2Fa%3Fb%26c%3Dd");
}

#[test]
fn structured_content_is_preferred_over_the_text_block() {
    let result = serde_json::json!({
        "content": [{ "type": "text", "text": "{\"redacted_text\":\"from text\"}" }],
        "structuredContent": { "redacted_text": "from structured" },
    });
    let payload = extract_tool_payload(&result);
    assert_eq!(payload["redacted_text"], "from structured");
}

#[test]
fn a_text_only_tool_result_is_parsed_as_json() {
    let result = serde_json::json!({
        "content": [{ "type": "text", "text": "{\"hits\":[1,2]}" }],
    });
    let payload = extract_tool_payload(&result);
    assert_eq!(payload["hits"][1], 2);
}

#[test]
fn a_result_without_content_falls_back_to_the_result_itself() {
    let result = serde_json::json!({ "ok": true });
    let payload = extract_tool_payload(&result);
    assert_eq!(payload["ok"], true);
}

#[test]
fn a_portfile_yields_the_addr_and_the_token() {
    let dir = test_temp_dir("portfile-with-token");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let portfile = dir.join("http.addr");
    std::fs::write(&portfile, "127.0.0.1:51786\nsecret-token\n").expect("write portfile");

    let endpoint = daemon::read_endpoint(&portfile).expect("read endpoint");
    assert_eq!(endpoint.addr, "127.0.0.1:51786");
    assert_eq!(endpoint.token, "secret-token");
    assert_eq!(endpoint.base_url(), "http://127.0.0.1:51786");
}

#[test]
fn a_portfile_without_a_token_is_refused() {
    let dir = test_temp_dir("portfile-no-token");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let portfile = dir.join("http.addr");
    std::fs::write(&portfile, "127.0.0.1:51786\n").expect("write portfile");

    // The token is the identity on a loopback socket, so a portfile without
    // one must not yield a usable endpoint.
    let error = daemon::read_endpoint(&portfile).expect_err("must refuse");
    assert!(
        error.to_string().contains("bearer token"),
        "the error must name the missing token, got {error}"
    );
}

#[test]
fn a_missing_portfile_names_the_daemon_start_command() {
    let dir = test_temp_dir("portfile-missing");
    let _ = std::fs::remove_dir_all(&dir);
    let portfile = dir.join("http.addr");
    let error = daemon::read_endpoint(&portfile).expect_err("must refuse");
    assert!(
        error.to_string().contains("basemind comms start"),
        "the error must tell the user how to start the daemon, got {error}"
    );
}

#[test]
fn tokens_become_entity_spans() {
    let mut tokens_for_entity = BTreeMap::new();
    tokens_for_entity.insert(
        "ENT_EMAIL_abcdef12".to_string(),
        vec!["[EMAIL_1]".to_string()],
    );
    let markdown = rewrite_tokens_as_spans("Contact [EMAIL_1] today.", &tokens_for_entity);
    assert_eq!(
        markdown,
        "Contact <span data-entity-type=\"EMAIL\" data-entity-id=\"ENT_EMAIL_abcdef12\">[EMAIL_1]</span> today."
    );
}

#[test]
fn every_token_of_one_entity_becomes_a_badge() {
    // The same value typed twice carries two tokens, and both must become
    // badges pointing at the same entity id.
    let mut tokens_for_entity = BTreeMap::new();
    tokens_for_entity.insert(
        "ENT_EMAIL_abcdef12".to_string(),
        vec!["[EMAIL_1]".to_string(), "[EMAIL_2]".to_string()],
    );
    let markdown = rewrite_tokens_as_spans("Mail [EMAIL_1] or [EMAIL_2].", &tokens_for_entity);
    assert_eq!(
        markdown
            .matches("data-entity-id=\"ENT_EMAIL_abcdef12\"")
            .count(),
        2
    );
}

#[test]
fn a_token_prefix_of_another_token_does_not_consume_it() {
    // `[EMAIL_1]` must not swallow the `[EMAIL_10]` next to it.
    let mut tokens_for_entity = BTreeMap::new();
    tokens_for_entity.insert(
        "ENT_EMAIL_abcdef12".to_string(),
        vec!["[EMAIL_1]".to_string()],
    );
    tokens_for_entity.insert(
        "ENT_EMAIL_99999999".to_string(),
        vec!["[EMAIL_10]".to_string()],
    );
    let markdown = rewrite_tokens_as_spans("Mail [EMAIL_10] and [EMAIL_1].", &tokens_for_entity);
    assert!(markdown.contains("data-entity-id=\"ENT_EMAIL_99999999\">[EMAIL_10]"));
    assert!(markdown.contains("data-entity-id=\"ENT_EMAIL_abcdef12\">[EMAIL_1]"));
}

#[test]
fn a_category_is_read_off_the_token_in_the_ui_vocabulary() {
    // The web UI's entity vocabulary and its `entityTypes.*` i18n keys are
    // upper case, so the category is normalized to that spelling.
    assert_eq!(category_of("[EMAIL_1]"), "EMAIL");
    assert_eq!(category_of("[IBAN_12]"), "IBAN");
}

/// The rehydration map must survive a restart: the desktop builds the store
/// per command, so the reveal runs against a store that never saw the
/// redaction.
#[test]
fn a_redaction_survives_a_new_store() {
    use openbitfun_basemind_integration::redaction::RedactionStore;

    let temp = test_temp_dir("persistence-test");
    let _ = std::fs::remove_dir_all(&temp);
    let registry = temp.join("registry");
    let vault = temp.join("vault");
    let payload = serde_json::json!({
        "redacted_text": "Contact [EMAIL_1].",
        "rehydration_map": { "[EMAIL_1]": "jean.dupont@example.com" },
    });

    let first = RedactionStore::with_passphrase(
        registry.clone(),
        vault.clone(),
        "workspace".to_string(),
        "test-passphrase".to_string(),
    )
    .expect("open the store");
    let document = first.record(&payload, "txt").expect("record the redaction");
    let entity_id = document
        .entities
        .first()
        .expect("one entity")
        .entity_id
        .clone();

    // A fresh store, as the desktop builds one per command.
    let second = RedactionStore::with_passphrase(
        registry,
        vault,
        "workspace".to_string(),
        "test-passphrase".to_string(),
    )
    .expect("reopen the store");
    let revealed = second
        .reveal(&entity_id)
        .expect("the entity must still reveal after a restart");
    assert_eq!(revealed, "jean.dupont@example.com");
}

/// A pseudonymised value (the fork's `pseudonyms` dictionary, ADR-0015) has
/// its token replaced in the text by the code name, while the rehydration map
/// still carries it. The store must keep the entity revealable, and must not
/// invent occurrences the text does not carry.
#[test]
fn a_pseudonymised_value_stays_revealable_without_a_token_in_the_text() {
    use openbitfun_basemind_integration::redaction::RedactionStore;

    let temp = test_temp_dir("pseudonym-test");
    let _ = std::fs::remove_dir_all(&temp);
    // The text says "Orme" because the caller's dictionary renamed the true
    // party; the map still holds the token, so the value stays reversible.
    let payload = serde_json::json!({
        "redacted_text": "Contact Orme today.",
        "rehydration_map": { "[PERSON_1]": "Jean Dupont" },
    });

    let store = RedactionStore::with_passphrase(
        temp.join("registry"),
        temp.join("vault"),
        "workspace".to_string(),
        "test-passphrase".to_string(),
    )
    .expect("open the store");
    let document = store.record(&payload, "txt").expect("record the redaction");

    let entity = document.entities.first().expect("one entity");
    assert_eq!(entity.entity_type, "PERSON");
    assert_eq!(entity.occurrence_count, 0);
    assert!(!document.markdown.contains("data-entity-id"));
    assert_eq!(
        store
            .reveal(&entity.entity_id)
            .expect("the pseudonymised value stays revealable"),
        "Jean Dupont"
    );
}

/// The fork's coverage contract (v0.34.1): `ner_truncated` refuses the payload
/// as a full redaction (D7), a value that is present but not a boolean is a
/// broken contract and refuses too, and an older sidecar that omits the field
/// passes.
#[test]
fn a_truncated_ner_pass_refuses_the_payload_but_a_v0_34_0_payload_passes() {
    use openbitfun_basemind_integration::redaction::ner_coverage_refusal;

    let truncated = serde_json::json!({ "ner_truncated": true, "ner_ran": true });
    assert!(ner_coverage_refusal(&truncated).is_some());

    let complete = serde_json::json!({ "ner_truncated": false, "ner_windows": 3 });
    assert!(ner_coverage_refusal(&complete).is_none());

    let older_sidecar = serde_json::json!({ "ner_ran": false });
    assert!(ner_coverage_refusal(&older_sidecar).is_none());

    // A broken coverage flag cannot be verified, so it refuses like a
    // truncation rather than being read as complete.
    let string_flag = serde_json::json!({ "ner_truncated": "yes" });
    let number_flag = serde_json::json!({ "ner_truncated": 1 });
    for payload in [string_flag, number_flag] {
        assert!(
            ner_coverage_refusal(&payload).is_some(),
            "a malformed coverage flag must refuse"
        );
    }
}
