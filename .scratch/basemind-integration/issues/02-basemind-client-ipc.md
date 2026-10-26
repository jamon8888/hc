# 02: BasemindClient + IPC (stdio JSON-RPC)

**What to build:** The `BasemindClient` in `ipc.rs` with stdio JSON-RPC transport. This is the single communication layer between Alex and the Basemind sidecar binary. Supports both local (sidecar) and remote (SSH-tunneled) modes.

**Blocked by:** 01 (crate skeleton)

**Status:** ready-for-agent

- [ ] Define `BasemindTransport` enum: `LocalStdio`, `LocalInProcess`, `RemoteSsh`
- [ ] Implement `BasemindClient::spawn_local()` — launches sidecar binary from Tauri sidecar path, connects via stdio
- [ ] Implement `BasemindClient::connect_remote(conn_id, remote_path)` — tunnels stdio over SSH using Alex's `remote_ssh` service
- [ ] Implement JSON-RPC 2.0 request/response framing with correlation IDs
- [ ] Implement core methods:
  - `extract_and_redact(bytes, path_hint) -> ExtractResult { markdown, entities[] }`
  - `extract_and_redact_path(path) -> ExtractResult`
  - `reveal_entity(entity_id) -> String`
  - `query_rag(query, top_k) -> Vec<RagHit>`
  - `detect_pii(text) -> Vec<PiiFinding>`
  - `ensure_models(models: &[ModelSpec]) -> Result`
- [ ] Add timeout handling (30s default) and retry logic
- [ ] Implement `BasemindClient::is_local_model_base_url()` (Rust port of TS logic)
- [ ] Unit tests with mock transport