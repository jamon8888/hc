# 01: Create `basemind-integration` crate skeleton

**What to build:** A new Cargo workspace member `src/crates/services/basemind-integration/` with the crate structure, feature flags, and empty module stubs. This is the foundation crate that isolates all Basemind logic from upstream code.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Create `Cargo.toml` with package metadata, edition 2024, workspace dependencies
- [ ] Define features: `basemind` (core), `tools-rag` (RAG tool), `document-read` (document conversion)
- [ ] Create `src/lib.rs` with module declarations and public exports
- [ ] Create empty module stubs:
  - `workspace_fs.rs` — `BasemindWorkspaceFs` decorator
  - `document_convert.rs` — `basemind_extract_and_redact()`
  - `prompt_pseudonym.rs` — `PromptPseudonymizer` + `SessionVault`
  - `session_vault.rs` — per-session encrypted vault
  - `local_model_detection.rs` — `is_local_model_base_url()`
  - `ipc.rs` — `BasemindClient` with stdio JSON-RPC
  - `models.rs` — model management (download, verify, cache)
- [ ] Add crate to workspace `Cargo.toml` members
- [ ] Verify `cargo check -p basemind-integration` passes