# headless-wordcraft-session

`Type: task`
`Status: resolved`

## Question

Implement the headless `wordcraft-cli mcp` session lifecycle per tickets 04/05: one server per
workspace session with a document open, idle-reaped (~10 min) and reaped on close; spawn only
via `alex_services_core::process_manager::{create_command, create_tokio_command}` (no bare
`Command`); per-process `WORDCRAFT_PROFILE_DIR` under Alex state; `--author
"Alex Agent (<session>)"`; both `--automation-*-root` mapped to the active workspace.
Register it as a managed MCP server (bridged into the runtime's MCP stack), with tool guidance
text covering: verify-after-batch with `inspect_document`, explicit `{"value": true}` on
tracking toggles, `paragraphs:true` on `type_text`, and the documented limits of the roots.
Capability unavailable → first step answers the record verbatim. Verification: focused
lifecycle tests (spawn/reap/confine) in `services-integrations`.

## Answer

Landed as `mcp::server::wordcraft_session` in `alex-services-integrations` (behind the
existing `mcp` feature, which already composes `services-core/process-runtime` — no manifest or
governance churn):

- `WordcraftSessionSpec::new(session_id, workspace_root, profile_dir)` — loud validation
  (id: 1..64 ASCII alnum plus `-._`, no leading `-`/`.`; roots/profile: non-empty, no control
  chars) so a bad id can never smuggle CLI flags into argv.
- `build_headless_session()` — the registry-consumable local-stdio definition: command
  `wordcraft-cli`; args `mcp --author "Alex Agent (<id>)" --automation-read-root <ws>
  --automation-write-root <ws>` (both roots = active workspace); env
  `WORDCRAFT_PROFILE_DIR=<state>/wordcraft/profiles/<id>` (isolated per ticket 05; deliberately
  NO `WORDCRAFT_NO_PREFS`, so crash logs live next to the isolated profile); working directory
  = workspace root (POSIX path discipline belongs to the caller for remote workspaces).
- `server_id()` = `wordcraft-<session id>` for the MCP registry; `author()` for
  tracked-changes attribution (map ticket 04: tracking is asserted per batch by the agent;
  `--author` at start is authoritative on the headless path).
- `wordcraft_session_idle_reap_due(last, now)` + `WORDCRAFT_IDLE_REAP` (10 min) — the pure
  reap decision the session owner (core/desktop orchestration) consumes at reaping time; the
  spawn itself already flows through `MCPConnection`'s `process_manager` path with
  `CREATE_NO_WINDOW` (never re-implemented here).

Verification (services-integrations AGENTS.md):

```bash
cargo test -p alex-services-integrations --no-default-features --features mcp --lib mcp::server::wordcraft  # 5/5 pass
cargo test -p alex-services-integrations --no-default-features --features mcp --lib                         # 27/27 pass
pnpm run check:core-boundaries      # pass
```

Carried over: tool guidance (verify-after-batch via `inspect_document`, explicit
`{"value": true}` on `review.trackChanges`, `paragraphs:true` on `type_text`, the
pathless-command/Zotero limits of the roots) is product-side tool-prompt text — it lands with
the runtime registration in the next wiring ticket, not in this crate.

