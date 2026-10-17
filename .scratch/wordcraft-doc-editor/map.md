# Map: wordcraft word-document integration

`Status: open`

## Destination

Bundled, working integration of [storytold/wordcraft](https://github.com/storytold/wordcraft) —
the open-source clean-room Word reimplementation (pure Rust, MIT OR Apache-2.0) — into
OpenBitFun: users can open and edit `.docx` documents in a WordCraft-backed editor surface, and
the agent can read, edit, and verify that document in response to user prompts, with the
architecture contract (distribution shape, editor surface, agent channels, process topology,
capability gate, remote stance) recorded the way `univer-cli-integration` did.

## Notes

- Domain: Word-compatible document viewing/editing/creation (`.docx` first-class; also
  `.odt/.rtf/.html/.md/.tex/.txt`; PDF/PNG export), agent-driven editing (MCP + control
  protocol), Web UI surface, process topology.
- Upstream speed brief (verified 2026-10-16 against README + `docs/mcp.md` + `docs/control-protocol.md`):
  - Pure Rust, layered crates L0 `wordcraft-geom` → L6 `wordcraft-ui-egui`, apps
    `wordcraft` (egui desktop), `wordcraft-cli`, `wordcraft-web` (wasm via trunk). No
    Electron/Tauri in upstream — UI is swappable by design.
  - Alpha: ~88% of ribbon features have commands, self-estimated ~60% to full Word replacement;
    tracked changes, comments, references, mail merge already present.
  - Agent-first: same ~270+ commands drive the ribbon, shortcuts, CLI, JSON control channel, and
    MCP server. `inspect_document`/`get_text` let agents verify without screenshots. Tracked
    changes + author naming give reviewable agent edits.
- License is MIT OR Apache-2.0 (permissive, vendor-friendly — unlike the univer Pro constraint).
  Attribution/asset licensing is still per-ticket (upstream ships its own attribution check).
- Repo rules that bind every ticket here: root `AGENTS.md` remote-scenarios (remote workspace /
  remote control / peer device / detached dispatch), upgrade-compatibility, and non-interactive
  child-process rules (`create_command` facade, `CREATE_NO_WINDOW`, `windowsHide`).
- Skills: `/grilling` + `/domain-modeling` for decision tickets; `/research` (subagent) for
  research tickets.
- Standing preference (carried over from the univer effort): loud degradation over silent
  fallback; no silent local fallbacks in any remote scenario.
- Tracker: local markdown (`.scratch/`), wayfinding dialect, per `docs/agents/issue-tracker.md`.

## Decisions so far

- [agent-channels](issues/02-agent-channels.md) — two channels over one ~270-command id space:
  headless `wordcraft-cli mcp` (primary agent channel; OpenBitFun already bridges MCP) and
  loopback JSON control protocol with key-file auth (secondary, for a live visible editor);
  tracked-changes author naming + `inspect_document` give the reviewable, screenshot-free agent
  editing loop.
- [upstream-distribution](issues/01-upstream-distribution.md) — PREBUILT + PINNED, resolved via
  `ManagedRuntimeResolver` (path → PATH → managed component, pinned tag, SHA256-verified,
  ~82–100 MB portable archives, downloaded at enable time); vendoring crates rejected for day
  one; attribution = unmodified binary + both license texts; pin bumps smoke-tested by ticket 07.
- [editor-ui-surface](issues/03-editor-ui-surface.md) — NATIVE APP WINDOW day one (upstream egui
  app, driven over the control protocol; headless MCP for invisible work), wordcraft-web embed as
  upgrade goal gated on the 07 probe; entries = "Open in WordCraft" on .docx + "New Docx";
  zero new theme tokens (WordCraft owns chrome); its own i18n; loud unsupported instead of a
  silent viewer fallback.
- [agent-interaction-contract](issues/04-agent-interaction-contract.md) — managed `wordcraft-cli
  mcp` server per workspace (unavailable → capability record verbatim); tracked changes always on
  authored `OpenBitFun Agent (<session>)`; live-if-open (MCP `--connect` bridge) else headless;
  agent saves after each batch.
- [process-topology](issues/05-process-topology.md) — headless server per workspace session with
  idle reap; per-process `WORDCRAFT_PROFILE_DIR` under OpenBitFun state + adapter-only key; both
  automation roots = active workspace; spawns only via `process_manager`;
  explicit path → PATH → managed component discovery.
- [remote-scenarios](issues/06-remote-scenarios.md) — remote workspace SUPPORTED (headless server
  on the workspace host, managed component provisioned there — scope expansion, provisioning
  design owed to 08); bots edit headless, visible window stays local; registry rows per root
  rules; detached dispatch supported via the headless path with capability ads.
- [e2e-spike](issues/07-e2e-spike.md) — PROVEN headless loop end to end on v0.7.0 (MCP handshake →
  track ON → type/select/bold → save → inspect; outside-root save refused; OOXML `w:ins` authored
  "OpenBitFun Agent"; independent CLI read-back). Visible-app leg environment-blocked (no GPU
  surface here) → real-device validation owed to 08; web zip is a static wasm hosting bundle
  (licenses, COOP/COEP headers, no control channel) — live embed probe still owed.
- [implementation-breakdown](issues/08-implementation-breakdown.md) — ticket set 09–16: capability
  record (09) → headless session (10) → live bridge (11) + editor entries (12);
  doc-surface-relationship (13) and remote provisioning (14) as grilling; registry rows (15);
  validation loop with real-GPU owed item (16). Verification at the owner tickets.
- [wordcraft-capability-record](issues/09-wordcraft-capability-record.md) — LANDED:
  `services-core::wordcraft` record + cached probe (available/unsupported/not-installed/error,
  Recheck backing, sha256 `SHA256SUMS.txt` verifier) + `wordcraft` managed component in
  `managed_runtime` + boundary-governance mirrors; 6/6 record tests, boundary checks green.
- [headless-wordcraft-session](issues/10-headless-wordcraft-session.md) — LANDED:
  `mcp::server::wordcraft_session` spec → local-stdio definition (author, both roots = workspace,
  isolated profile dir, cwd = workspace) + idle-reap decision; 5/5 + family tests green.
- [live-if-open-bridge](issues/11-live-if-open-bridge.md) — LANDED: validated `--connect` bridge
  spec (key-as-env only, no roots, no profile env) + pure single-writer route
  (live / headless / loud-refused); 8/8 module tests, family 30/30, boundaries green.
- [editor-surface-entries](issues/12-editor-surface-entries.md) — LANDED: desktop
  `wordcraft_status_command` (shared record, Recheck) + `wordcraft_open_document_command`
  (resolve → .docx/local-only loud refusals → free loopback port + isolated profile →
  process_manager spawn outside the managed tree) + web-ui context entries
  (`.docx` open + new-doc start screen) with loud remediation toasts and i18n keys
  (ticket sketch's "no i18n" overridden by repo i18n policy); registry rows LocalOnly/ControllerLocal.
  Verification: desktop `wordcraft` tests 4/4 (a real uniqueness bug — two opens in one
  process sharing a pid-only fingerprint — caught and fixed with a per-process sequence in
  the profile-dir fingerprint), `remote_workspace_policy` closure clean for wordcraft rows
  (its only current failure set is the parallel basemind effort's unrow'd commands),
  `check:web` green end-to-end (vite build + tsc), focused vitest suites green.
- [registry-and-capability-ads](issues/15-registry-and-capability-ads.md) — RESOLVED: exactly
  two Tauri commands, both rows landed (`wordcraft_open_document_command` LocalOnly/
  ControllerLocal/REFUSED, `wordcraft_status_command` LocalOnly/ControllerLocal/
  CLI_NOT_IMPLEMENTED); "new docx" reuses the open command with an empty path (no third
  command); headless MCP session + live bridge + web adapter are services/no-new-command
  surfaces recorded verbatim; `check:core-boundaries` and `check:repo-hygiene` green.
- [doc-surface-relationship](issues/13-doc-surface-relationship.md) — RESOLVED: complement
  with format ownership — one tool-owned entry family per format (docx → wordcraft,
  xlsx/csv/pptx → univer; the docx overlap with univer Doc resolves to wordcraft for the
  file-explorer menu), separate capability gating (tool-named entries never re-pointed, no
  aggregate record, no availability auto-pick), anydoc stays the agent read lane for every
  format, shared entry-family extraction deferred until univer entries land with an explicit
  criterion; new ticket 17 for the docx double-click → code-editor silent-dump gap.
- [remote-provisioning-design](issues/14-remote-provisioning-design.md) — RESOLVED:
  provision-per-host, controller-pushed — target-platform asset fetched+SHA256-verified on the
  controller, flashgrep two-phase transfer (write_file → tmp → mv → remote sha re-check → tar
  extract → `--version` verify) into a host-managed POSIX runtime root (version-scoped dirs +
  `current`, per-session profiles; rejected workspace-scoped for a 100 MB workspace-independent
  tool), resolution mirrored over the transport (explicit path → PATH → managed) feeding the
  same `WordcraftCapability` vocabulary, headless MCP via exec-channel stdio bridge on
  flashgrep's remote-stdio session machinery (idle-reap + trap teardown carried from 05),
  provisioning as a named visible operation (never a hidden turn side effect), acceptance
  table per target (Windows SSH loud-unsupported day one, dispatch via negotiated capability
  ads per `supports_provisioning` precedent, bot window explicit-unsupported), remote version
  floor negotiates (at-or-above floor = available), `ssh -L` stays documentation-only; all
  remote-behavior evidence owed to 16.
- Grilling round 2 (2026-10-17) amendments recorded in 13 + 14: WordCraft gets `.docx` on
  default-open when the record is `available` (ticket 17 routing stands); last-write-wins
  accepted across concurrent editors, no interlock; remote provisioning fires automatically at
  session/task start (still loud/visible); remote process model becomes a **persistent
  per-user-per-workspace host daemon** (attach/detach + identity re-validation, no idle reap
  on the host — amends 14 §4/§7; desktop headless keeps its reap).
- [docx-double-click-routing](issues/17-docx-double-click-routing.md) — LANDED:
  `FileTabManager.openFile` (single funnel: double-click / file:open / openFileInBestTarget)
  routes `.docx` (no explicit editorType) through `statusQuiet()`: available → WordCraft
  window, other record states → loud remediation toast, never the code-editor dump;
  probe-unavailable surfaces keep the tab (the recorded carve-out); explicit "open as text"
  bypasses; 6/6 focused tests, no new i18n keys or registry rows.
- [validation-loop](issues/16-validation-loop.md) — RESOLVED: every locally runnable leg green
  — real docx round trip PASS in strict mode against `wordcraft-cli 0.7.0` (tracked insertions
  authored as the agent, bold run, independent read-back, outside-write-root refusal), web-ui
  focused tests 13/13 incl. loud-unavailable paths, crate tests 6/6 + 30/30 + 4/4, boundaries /
  fmt / check:web green, i18n parity zero-delta vs the pre-existing red baseline; the loop
  found and fixed two real bugs (checksum tests' missing `PathBuf` import; profile-dir
  fingerprint colliding across opens in one process — monotonic per-process sequence added).
  Owed with owners: real-GPU visible-app leg + embed probe → device hardware (same carry as
  07); full detached-dispatch e2e → blocked on 14's provisioning implementation, with the
  no-interactive-host headless core already proven by the round trip.
<!-- one line per resolved ticket: gist + link -->

## Not yet specified

- All 17 tickets resolved. Carries that live outside the tracker: real-GPU visible-app
  validation + wordcraft-web embed probe (owed to macOS/Windows device hardware, same owner
  as ticket 07's carry), and the full detached-dispatch e2e (blocked on ticket 14's
  provisioning implementation landing; the headless no-interactive-host core is proven).
  Everything in this tracker is still uncommitted in the working tree.

## Out of scope

- Changing upstream WordCraft behavior or forking it (we consume it as a clean-room, permissively
  licensed dependency).
- Feature parity chasing with Word (upstream's own roadmap).
