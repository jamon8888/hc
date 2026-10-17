# editor-surface-entries

`Type: task`
`Status: resolved`

## Question

Ship the day-one editor surface (ticket 03): spawn the upstream egui `wordcraft` app as its own
OS window via `process_manager` with isolated `WORDCRAFT_PROFILE_DIR`, opened as
*ControllerLocal* for the entry points — **"Open in WordCraft"** on `.docx` (`.odt`/`.rtf` later)
in the file tree, and a **"New Docx document"** create entry. Entries visible-but-gated on the
capability record (reason + remediation + Recheck; loud unsupported on no GPU/binary). No new
panel type, no theme tokens (WordCraft owns its chrome), no OpenBitFun i18n keys. Touch points:
web-ui file-tree context menu + creation flow (see
`src/web-ui/src/shared/types/panelContent.ts` / PanelController context), desktop adapter for
the spawn. State which remote scenarios were exercised (peer-device: launch is
ControllerLocal).

## Answer

Landed in two halves — the desktop spawn command and the web-ui entries.

**Desktop adapter — `src/apps/desktop/src/api/wordcraft_api.rs` (new):**

- `wordcraft_status_command(recheck)` — returns the single factual
  `WordcraftCapability` record from `openbitfun_services_core::wordcraft`
  (explicit path → system PATH → managed component). `recheck: true` forces a
  fresh probe; the probe instance is a process-wide `OnceLock` rooted at the
  path manager's managed-runtimes dir, so UI entries and the agent read the
  same answer.
- `wordcraft_open_document_command(request)`:
  1. Legacy `connectionId` pointer → refused loudly up front.
  2. Path resolution via the shared `resolve_desktop_path_target`
     (`src/api/path_target.rs`): workspace-relative and `openbitfun://`
     runtime-URI forms both resolve; `Remote` targets refuse loudly ("remote
     provisioning not supported yet") — no silent local fallback.
  3. Document must exist as a file with a `.docx` extension (day-one scope;
     case-insensitive; `.rtf` refused with an explicit message).
  4. Capability must be `Available`, else the record's status + message +
     remediation are surfaced verbatim. A CLI-only install refuses the window
     with an explicit "agent editing still works; install the full build" —
     the visible editor needs the `wordcraft` app binary, not just
     `wordcraft-cli`.
  5. Binds a free loopback port (`127.0.0.1:0`) for the control channel and
     creates an isolated profile dir
     `<user_data>/wordcraft/profiles/<stem>-<sha256[:12]-pid>`; both are
     passed to the app (`--control <port>`, `WORDCRAFT_PROFILE_DIR`). The key
     file lands inside our isolated profile (`control-key.local-<port>`),
     never logged, never exposed to the agent.
  6. Spawn via `openbitfun_core::util::process_manager::create_command`
     (CREATE_NO_WINDOW-safe façade, per repo policy) *outside* the managed
     child tree: the editor window is user-facing and outlives OpenBitFun;
     killing OpenBitFun must not kill the user's editor.
- Registry rows (ticket 15 seam): `wordcraft_open_document_command` =
  `LocalOnly`, `ControllerLocal`, `REFUSED` (CLI peer never spawns a window);
  `wordcraft_status_command` = `LocalOnly`, `ControllerLocal`,
  `CLI_NOT_IMPLEMENTED`. Rows inserted in sorted position (before
  `workspace_…`, the sort assertion caught me on 'd' < 'k').

**Web UI — context menu + commands:**

- `WordcraftAPI.ts` service-API adapter (`wordcraft_status_command`,
  `wordcraft_open_document_command`), invoke-through-adapters per the
  no-direct-Tauri rule; `statusQuiet()` for best-effort availability reads.
- `WordcraftDocumentCommands.ts`: `OpenInWordcraftCommand`
  (`file.open-in-wordcraft`, `.docx` files only) and
  `NewWordcraftDocumentCommand` (`file.new-wordcraft-document`). Both reuse
  `canRevealInExplorer`'s local-surface guard (no remote paths, active surface
  only) and surface failures as a notification with the backend remediation
  verbatim next to a generic title — loud degradation with no silent retry.
- `FileExplorerMenuProvider`: "Open in WordCraft" appears on `.docx` file
  context (visible but `disabled` for remote scopes, matching the HTML-browser
  entry's pattern), and "New WordCraft document" is a third item in the
  folder/submenu "New" group. New-doc delegates document creation to
  WordCraft's own start screen (blank/templates/save-location stay in its
  chrome — ticket 03's "WordCraft owns the UI" decision).
- Labels: the ticket sketch said "no i18n keys", but the repo-level Web UI
  policy (`i18nService.t` for all labels, `pnpm run i18n:audit`) overrides it;
  six `common:file.*` keys + one `errors:file.*` key were added to all four
  locales (en-US, fr-FR, zh-CN, zh-TW). Theme tokens / panel types: untouched,
  as planned.

**Recheck UX (day one):** the menu item does not pre-query status (async
provider menus shouldn't they are invoked per-open); gating is enforced at
launch time with the full record + remediation in the failure toast, and
`wordcraft_status_command { recheck: true }` exists behind the same record for
a settings-tier Recheck entry (tickets 13/15 will place it). Availableness is
never fabricated.

**Remote scenarios exercised:** none end-to-end — desktop-local slice only.
Peer Device Mode the launch route is `ControllerLocal` by registry row; the
CLI peer route is refused (`REFUSED`) and `CLI_NOT_IMPLEMENTED`; remote
workspace is intentionally `LocalOnly` with a loud refusal message until
ticket 14 designs host provisioning. Headless server-side scenario stays
covered by tickets 10/11 specs, not this command.

### Verification (closed during ticket 15/16 checks)

- `cargo test -p openbitfun-desktop --lib wordcraft` — 4/4 pass
  (raw-input rejection, local document validation with case-insensitive
  `.docx`, loopback port scan, per-session profile-dir uniqueness; the
  uniqueness test caught a real collide-includes-dir bug where two opens in
  one process shared pid-only fingerprint — fixed with a monotonic per-process
  sequence in the dir fingerprint).
- `cargo test -p openbitfun-desktop --lib remote_workspace_policy` — the
  command rows for wordcraft parse clean (no wordcraft entry appears in the
  closure failure set). The closure test currently fails on
  `basemind_read_redacted_document` / `basemind_reveal_entity`, registered by
  the parallel basemind effort without their ops rows yet — that failure
  belongs to that effort's ticket, not this one.
- Web UI: `WordcraftDocumentCommands.test.ts` 7/7, `RevealInExplorerCommand.test.ts`
  13/13 (regression), `check:web` green end-to-end (vite build + tsc; the
  `@/generated/api` gap was closed by running `pnpm --dir src/web-ui run gen:types`).
- `pnpm run i18n:audit` remains red on this tree for pre-existing reasons
  (missing `fr-FR/flow-chat.json` at HEAD, stale shared-term baselines) —
  my locale additions add no new candidate entries.

