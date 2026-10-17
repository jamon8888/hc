# docx-double-click-routing

`Type: task`
`Status: resolved`

Blocked by: 12, 13

## Question

Fix the gap ticket 13 uncovered: double-click / `file:open` on a `.docx` currently routes to
`code-editor` via `getEditorType` (`src/web-ui/src/infrastructure/language-detection/utils/helpers.ts`)
— a silent binary dump, forbidden by decision 03 once WordCraft is a registered surface.
Route `.docx` opens through the same capability-aware decision as the context entry:
capability `available` → launch the WordCraft window (reuse `wordcraftAPI.openDocument`,
same action as `file.open-in-wordcraft`); capability `not-installed`/`unsupported`/`error`
→ loud record state + remediation toast, never the code-editor dump; remote workspace target
→ loud local-only refusal (registry `LocalOnly`/`REFUSED` row).

Do **not** patch `getEditorType` with a WordCraft dependency — it is a pure extension→panel
mapper shared by every surface (including server/web where no desktop host adapter exists).
The routing decision belongs at the `file:open` / `openFileInBestTarget` seam (or the
`FilesPanel.handleOpenFile` entry), behind the existing `WordcraftAPI.statusQuiet()` probe,
and must stay a no-op on surfaces where the probe is unavailable (answer: keep current
behavior there only if the loud path is also unavailable — record which in the Answer).

Coverage: `FileExplorerMenuProvider.test.ts`-style contract for the new routing,
web-ui `pnpm run check:web`, and the i18n keys for any new copy
(`pnpm run i18n:audit`). If the routing adds Tauri invocations, confirm the existing
registry rows cover it — no new command is expected (reuses
`wordcraft_open_document_command`).

## Answer

Landed at the **single dispatch seam** the ticket asked for: `FileTabManager.openFile`
(`src/web-ui/src/shared/services/FileTabManager.ts`) — which is the shared funnel for
double-click, `file:open`, and `openFileInBestTarget`, so FilesPanel, tree items, and chat
file cards all take the same route. No new registry rows (the route reuses
`wordcraft_open_document_command`'s row; ticket 15's recorded row set stands).

Routing (`openWordcraftOrLoudFallback`):

- Fires only for `.docx` (`isWordcraftDocumentPath`, from ticket 12's commands module) and
  only when **no explicit `editorType` was chosen** — "Open as Text" / html-preview-style
  explicit targets bypass the route entirely, keeping the user-override path intact.
- `getEditorType` itself is untouched (pure mapper preserved for every surface).
- `wordcraftAPI.statusQuiet()` gates the decision:
  - **probe unavailable** (server/web surfaces with no desktop host adapter) → the previous
    tab behavior kept unchanged. This is the only "keep current behavior" branch and it is
    *the recorded one*: there the loud path is also unavailable (no host adapter to answer
    the record), which is exactly the carve-out the ticket licensed.
  - **`not-installed` / `unsupported` / `error`** → loud toast with the record's
    remediation/cause verbatim (`errors:file.wordcraftOpenFailed` title), **no tab opened** —
    the code-editor binary dump is gone even below capability.
  - **`available`** (and an `app` binary — the visible editor) → `wordcraftAPI.openDocument`
    with the normalized path + scope workspace ID. Launch failures (incl. remote-target
    refusals from the desktop command, which answers its `LocalOnly` row loudly) surface the
    backend message verbatim in the toast; again no tab.
- Diagnostic logging stays English-only through `createLogger('FileTabManager')`.

Round-2 note (this turn): ticket 13's grilling confirmed the endgame — when the record is
`available`, docx default-open **is** WordCraft, which this routing now implements; the
fallback viewer keeps existing only for explicit "open as text" actions and other formats.

Coverage: `src/shared/services/FileTabManager.test.ts` — 6 focused tests (available →
window + no tab; not-installed → loud toast + no tab; launch failure → verbatim backend
message + no tab; probe unavailable → tab kept; explicit `editorType` bypass with zero
invokes; non-docx files untouched). 6/6 pass locally; full-suite gates (`check:web`) are
tracked at the turn's verification section, and `pnpm run i18n:audit` copy: **no new keys**
(reused `errors:file.wordcraftOpenFailed` from ticket 12, already present in all four
locales).
