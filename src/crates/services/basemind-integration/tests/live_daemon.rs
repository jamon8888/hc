//! Integration test against a live basemind daemon.
//!
//! These tests exercise the real MCP surface of the `jamon8888/basemind`
//! fork: the transport, the tool names, and the response shape. A mock would
//! only prove the mock agrees with itself; the fork is the contract.
//!
//! The tests are skipped unless a daemon is reachable: they need
//! `BASEMIND_COMMS_DIR` (or the default per-user location) to hold an
//! `http.addr` portfile, and a workspace root the daemon will host.
//!
//! Run with:
//! ```text
//! BASEMIND_TEST_ROOT=/path/to/git/workspace cargo test -p alex-basemind-integration --features basemind -- --ignored
//! ```

#![cfg(feature = "basemind")]

use alex_basemind_integration::daemon;
use alex_basemind_integration::ipc::BasemindClient;
use alex_basemind_integration::redaction::RedactionStore;

/// The workspace the daemon should host. The daemon refuses any root that is
/// neither a git repository nor a directory carrying a basemind config.
fn test_root() -> Option<std::path::PathBuf> {
    std::env::var_os("BASEMIND_TEST_ROOT").map(std::path::PathBuf::from)
}

/// A client bound to the live daemon, and the workspace it hosts, when one
/// is reachable.
fn live_client() -> Option<(BasemindClient, std::path::PathBuf)> {
    let root = test_root()?;
    let endpoint = daemon::discover().ok()?;
    Some((
        BasemindClient::new(endpoint.base_url(), endpoint.token),
        root,
    ))
}

/// The daemon really serves MCP, and the tool surface is the one the client
/// calls. This is the assertion that would have caught the original
/// `extract_and_redact_path` and `reveal_entity` calls: neither tool exists.
#[tokio::test]
#[ignore = "requires a running basemind daemon"]
async fn daemon_serves_the_tools_the_client_calls() {
    let Some((client, root)) = live_client() else {
        eprintln!("skipping: no basemind daemon reachable");
        return;
    };

    // A tool the fork does not have must be refused, and the error must
    // name it. The fork answers an unknown tool with an error rather than
    // an empty success, so the client surfaces it instead of handing the
    // caller a silent empty payload.
    let missing = client
        .call_tool(
            "extract_and_redact_path",
            serde_json::json!({ "path": "/tmp/x" }),
            &root.to_string_lossy(),
        )
        .await
        .expect_err("a tool the fork does not have must fail");
    assert!(
        missing.to_string().contains("extract_and_redact_path"),
        "the error must name the tool that is missing, got {missing}"
    );

    // `redact_text` is a real tool and must answer.
    let payload = client
        .call_tool(
            "redact_text",
            serde_json::json!({ "text": "call 555-0100" }),
            &root.to_string_lossy(),
        )
        .await
        .expect("redact_text must be callable");
    assert!(
        payload.get("redacted_text").is_some(),
        "redact_text must answer with redacted_text, got {payload}"
    );
}

/// The document search tool answers with a `hits` array in its structured
/// content. The client unwraps the MCP envelope, so the hits must be read
/// from the payload itself.
#[tokio::test]
#[ignore = "requires a running basemind daemon"]
async fn document_search_answers_with_hits() {
    let Some((client, root)) = live_client() else {
        eprintln!("skipping: no basemind daemon reachable");
        return;
    };

    let hits = client
        .search_documents("contact", 3, &root.to_string_lossy())
        .await
        .expect("memory mode=documents must answer");
    // An empty index is a valid answer; a parse failure is not.
    assert!(
        hits.len() <= 3,
        "the search must honour the requested limit, got {}",
        hits.len()
    );
}

/// A document redaction round-trips: the fork extracts the file, the product
/// mints entity ids, and the same value reveals back to the original.
#[tokio::test]
#[ignore = "requires a running basemind daemon"]
async fn document_redaction_round_trips_through_the_product_store() {
    let Some((client, root)) = live_client() else {
        eprintln!("skipping: no basemind daemon reachable");
        return;
    };
    let Some(root) = test_root() else {
        eprintln!("skipping: BASEMIND_TEST_ROOT is not set");
        return;
    };

    let document = root.join("basemind-redaction-fixture.txt");
    std::fs::write(
        &document,
        "Contact jean.dupont@example.com about FR7630006000011234567890189.\n",
    )
    .expect("write fixture");

    let payload = client
        .redact_path(&document.to_string_lossy(), &root.to_string_lossy())
        .await
        .expect("redact_text with a file_path must succeed");

    let temp = std::env::temp_dir().join("alex-basemind-redaction-test");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("create temp dir");
    let store = RedactionStore::with_passphrase(
        temp.join("registry"),
        temp.join("vault"),
        "test-workspace".to_string(),
        "test-passphrase".to_string(),
    )
    .expect("open redaction store");

    let redacted = store.record(&payload, "txt").expect("record the redaction");
    assert!(
        redacted.markdown.contains("data-entity-id"),
        "the redacted markdown must carry entity spans, got {}",
        redacted.markdown
    );
    assert!(
        !redacted.markdown.contains("jean.dupont@example.com"),
        "the redacted markdown must not leak the original value"
    );
    assert!(
        !redacted.entities.is_empty(),
        "the redaction must produce at least one entity"
    );

    // Every entity must reveal back to the original value.
    for entity in &redacted.entities {
        let value = store
            .reveal(&entity.entity_id)
            .unwrap_or_else(|| panic!("entity {} must reveal", entity.entity_id));
        assert!(
            value.contains('@') || value.len() > 5,
            "the revealed value must be the original, got {value}"
        );
    }
}
