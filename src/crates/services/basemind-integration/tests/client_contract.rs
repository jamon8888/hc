//! Unit tests for the pure parts of the basemind integration.
//!
//! These cover the pieces that decide what the product sends to the fork and
//! how it reads the answer back: the query-string encoding, the MCP result
//! unwrapping, the portfile parsing, and the token-to-span rewrite.

#![cfg(feature = "basemind")]

use openbitfun_basemind_integration::daemon;
use openbitfun_basemind_integration::ipc::testable::{extract_tool_payload, urlencoding};
use std::collections::BTreeMap;

#[test]
fn query_values_are_percent_encoded() {
    // The daemon decodes the query with `form_urlencoded`, so a percent-encoded
    // separator comes back as a separator: escaping it is harmless, and
    // escaping the characters that would otherwise change the query's meaning
    // is what matters.
    assert_eq!(urlencoding("/tmp/plain"), "%2Ftmp%2Fplain");
    assert_eq!(urlencoding("/tmp/with space"), "%2Ftmp%2Fwith%20space");
    assert_eq!(
        urlencoding("/tmp/Users/jamín/文档"),
        "%2Ftmp%2FUsers%2Fjam%C3%ADn%2F%E6%96%87%E6%A1%A3"
    );
    // Characters that carry meaning in a query string must not survive raw.
    assert_eq!(urlencoding("/a?b&c=d"), "%2Fa%3Fb%26c%3Dd");
    // A separator stays a separator once the daemon decodes it.
    assert_eq!(
        urlencoding("/tmp/my repo"),
        "%2Ftmp%2Fmy%20repo",
        "the encoded form must decode back to the original path"
    );
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
    let dir = std::env::temp_dir().join("openbitfun-basemind-portfile-with-token");
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
    let dir = std::env::temp_dir().join("openbitfun-basemind-portfile-no-token");
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
    let dir = std::env::temp_dir().join("openbitfun-basemind-portfile-missing");
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
    let markdown = openbitfun_basemind_integration::redaction::testable::rewrite_tokens_as_spans(
        "Contact [EMAIL_1] today.",
        &tokens_for_entity,
    );
    assert_eq!(
        markdown,
        "Contact <span data-entity-type=\"email\" data-entity-id=\"ENT_EMAIL_abcdef12\">[EMAIL_1]</span> today."
    );
}

#[test]
fn every_token_of_an_entity_becomes_a_span() {
    let mut tokens_for_entity = BTreeMap::new();
    tokens_for_entity.insert(
        "ENT_EMAIL_abcdef12".to_string(),
        vec!["[EMAIL_1]".to_string(), "[EMAIL_2]".to_string()],
    );
    let markdown = openbitfun_basemind_integration::redaction::testable::rewrite_tokens_as_spans(
        "Contact [EMAIL_1] and [EMAIL_2] today.",
        &tokens_for_entity,
    );
    assert_eq!(
        markdown,
        "Contact <span data-entity-type=\"email\" data-entity-id=\"ENT_EMAIL_abcdef12\">[EMAIL_1]</span> and <span data-entity-type=\"email\" data-entity-id=\"ENT_EMAIL_abcdef12\">[EMAIL_2]</span> today."
    );
}

#[test]
fn a_category_is_read_off_the_token() {
    assert_eq!(
        openbitfun_basemind_integration::redaction::testable::category_of("[EMAIL_1]"),
        "email"
    );
    assert_eq!(
        openbitfun_basemind_integration::redaction::testable::category_of("[IBAN_12]"),
        "iban"
    );
}
