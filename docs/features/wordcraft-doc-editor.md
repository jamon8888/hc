# WordCraft document editor

WordCraft ([storytold/wordcraft](https://github.com/storytold/wordcraft), MIT/Apache-2.0
upstream) is Alex's Word/docx editor. The agent edits documents **on user prompt**;
tracked changes are always authored `Alex Agent (<session>)`. Design decisions live in
`.scratch/wordcraft-doc-editor/` (map + ticket answers) — this document records the stable
product contract only.

## Capability record (single source of truth)

`openbitfun_services_core::wordcraft::WordcraftProbe` (desktop-enabling feature `wordcraft`)
produces one `WordcraftCapability` record, resolved **explicit configured path → system PATH →
managed component** (`wordcraft` component under the managed runtime root; component version
pinned at `PINNED_WORDCRAFT_VERSION`). Statuses: `available | unsupported | not-installed |
error`; `message`/`remediation` are user-actionable text; `recheck()` forces fresh detection.
Every surface — Tauri command, Web UI adapter, agent tool guidance — answers **verbatim from
this record**; no surface re-derives status, and availability is never fabricated.

Distribution is the pinned upstream release archive verified against upstream `SHA256SUMS.txt`
(never vendored crates, never unverified binaries). The `wordcraft-web` static bundle
(COOP/COEP headers, no control channel) is reserved for a later same-origin embed upgrade; live
probe owed before that path commits.

## Editor surface (desktop)

- `wordcraft_status_command(recheck)` — the record above; backs UI cards and Recheck actions.
- `wordcraft_open_document_command({ path?, workspaceId? })` — spawns the upstream egui app as
  its own OS window:
  - Path resolution via the shared desktop resolver (workspace-relative and runtime-URI forms);
    **remote targets refuse loudly** ("local workspace only … not supported yet") until the
    remote host design lands (see Remote below).
  - Day-one docx scope: `.docx` only; other formats refused with an explicit message.
  - Spawn carries a free loopback control port (`--control`) and an isolated
    `WORDCRAFT_PROFILE_DIR` under `user data/wordcraft/profiles/<stem>-<fingerprint>`; the
    per-launch control key file (`control-key.local-<port>`) lives in that profile and is
    never logged or exposed to the agent.
  - The editor is spawned **outside the managed child tree**: it is user-facing and outlives
    Alex; app teardown must not kill the user's editor.
  - CLI-only installations refuse the window with "agent editing still works" — the visible
    editor needs the `wordcraft` app binary, not just `wordcraft-cli`.
- Registry stances (`remote_surface/table.rs`): both commands `LocalOnly`, `ControllerLocal`;
  the open command is `REFUSED` on the CLI peer, status is `CLI_NOT_IMPLEMENTED`.
- Web UI: `Open in WordCraft` on `.docx` context menus, `New WordCraft document` (delegates
  creation to WordCraft's own start screen/templates), and **`.docx` default-open routes to
  WordCraft when the record is `available`** (`FileTabManager` seam; explicit "open as text"
  bypasses; other record states fail loudly with the record's remediation instead of the old
  binary-dump viewer). Concurrent editors use last-write-wins by decision; no interlock.

## Agent editing channels

Routing (`mcp::server::wordcraft_session::wordcraft_route_session`) follows the single-writer
rule:

- **Headless (primary)**: `wordcraft-cli mcp --author "Alex Agent (<session>)"
  --automation-read-root <workspace> --automation-write-root <workspace>`, isolated profile,
  cwd = workspace. Saves outside the roots are refused by upstream's automation gate (readable
  error, document unchanged). Idle reap: `wordcraft_session_idle_reap_due` (10 min policy)
  applies to desktop headless sessions.
- **Live bridge**: when the document is open in the visible app and its control channel is
  reachable, the bridge runs `mcp --connect 127.0.0.1:<port>` with `WORDCRAFT_CONTROL_KEY`
  (loopback only; key read from the session profile). If the doc is open but the bridge is
  gone, the router **refuses loudly** rather than racing another writer.
- `document_open_in_visible_app` is the desktop adapter's opinion driving the route.

### Tool guidance for the agent (upstream MCP contract facts)

- The MCP surface exposes 16 tools; the ~270 editor commands ride the generic `execute`
  (`{command, params}`) and `batch` (`{commands:[...], keepGoing}`) tools. Dot-command ids
  come from `list_commands` (e.g. `select.text {text, occurrence}`, `format.bold {value}`).
- `type_text` does **not** split paragraphs on `\n` — pass `{text, paragraphs: true}` or send
  explicit `text.newParagraph` commands.
- Always pass `review.trackChanges {value: true}` — a bare toggle-style call can turn tracking
  **off**.
- Failed commands leave the document unchanged (verify-after-batch works on dirty state too);
  verify with `inspect_document` plus an independent read-back (`wordcraft-cli text <docx>`).
- Pathless hosting context: no Zotero-style document store integration day one.

## Remote scenarios

- **Remote workspace hosts** (SSH/jump/container): designed in the effort map (ticket 14) and
  amended by grilling — provisioning fetches the pinned *target-platform* archive
  controller-side (SHA256SUMS-verified), pushes it over the workspace transport (flashgrep
  two-phase pattern) into a host-managed POSIX runtime root (`wordcraft/<version>/` +
  `current`), and resolution re-runs the same ladder over the connection feeding the same
  `WordcraftCapability` vocabulary. Provisioning fires at session/task start (visible, loud).
  Process model (amended): a **persistent per-user-per-workspace host daemon** shared by
  sessions — attach/detach with identity re-validation (pinned version, roots, author),
  survives disconnects and app restarts, no remote idle reap. Until the remote host bits land,
  the desktop command refuses remote paths loudly; it never falls back to local editing of
  remote content.
- **Remote control** (mobile/bot): headless edits ride the host path; the visible window is
  answered with an explicit unsupported reply per its `ControllerLocal` stance.
- **Detached dispatch**: a docx-editing job requires the negotiated wordcraft capability ad
  (dispatch `supports_provisioning` precedent) — never assumed; missing ad refuses up front.
- POSIX-path discipline: remote roots are POSIX strings assembled with the remote join/escape
  helpers only; host `std::path` semantics never apply to them.

## Verification

Choose at the owner, never a mono-suite:

- Rust, in-crate: `cargo test -p openbitfun-desktop --lib wordcraft` (surface validation),
  `cargo test -p openbitfun-services-integrations --no-default-features --features mcp --lib
  mcp::server::wordcraft` (session spec + routing), `cargo test -p openbitfun-services-core
  --no-default-features --features wordcraft --lib wordcraft` (record/probe/checksums).
- Registry: `cargo test -p openbitfun-desktop --lib remote_workspace_policy`; governance:
  `pnpm run check:core-boundaries`, `pnpm run fmt:rs`.
- **Device gate (on demand, real binary required)**:
  `node tests/e2e/scripts/wordcraft-doc-roundtrip.mjs [--cli <path>] [--strict]` —
  headless loop: initialize → tracked edits via `execute`/`batch` → save → OOXML assertion
  (`w:ins` authored as the session's agent, bold run present) → independent `text` read-back →
  outside-write-root refusal. Loudly refuses to run without a resolvable binary.
- Owed to hardware (current machine cannot): visible-app control-channel validation on a
  real GPU surface — `wordcraft --control <port>` + key file auth, `engine.execute`,
  `screenshot` — and the optional wordcraft-web same-origin embed live probe. Run the checks
  above on macOS/Windows device hardware before claiming the visible leg validated.

## Out of scope

- Forking or modifying upstream WordCraft; we consume clean-room releases.
- Repointing tool-named entries across efforts (univer owns sheets/presentations; WordCraft
  owns `.docx`; the overlap is resolved in favor of WordCraft for context entries).
- Feature parity chasing with Word (upstream roadmap owns that).
