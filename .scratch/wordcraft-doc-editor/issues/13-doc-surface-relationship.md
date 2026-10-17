# doc-surface-relationship

`Type: grilling`
`Status: resolved`

## Question

Resolve the map's open "relationship with existing doc surfaces": how does WordCraft's docx
editing relate to the meditor markdown editor, anydoc conversions, and the parallel
univer-cli-integration effort — complement (docx = WordCraft, md = meditor, sheets = univer
pending), shared "Open in external tool" entry family, or deeper seams? Deliver: entry-match
table per format, no double-registration of the same file flow, and shared-vs-separate
capability gating where both external-tool entries exist.

## Answer

**Decision: complement with format ownership — one tool-owned context-entry family per
format, separate capability gating, no shared entry family yet, anydoc stays the agent read
lane.** Deeper seams (shared "Open in external tool" submenu, aggregate capability record,
cross-tool routing) are rejected for now; extraction criteria recorded below.

### Entry-match table (current state, per format)

| Format | Built-in tab surface | Context entries (owner) | Double-click today | Agent read | Agent edit | New doc |
|---|---|---|---|---|---|---|
| `.md` / `.plan.md` | meditor tab (`markdown-editor`) / `plan-viewer` | none (built-in edit) | meditor tab | native text | file tools (text) | built-in New file |
| `.html`/`.htm` | `html-preview` tab | `file.open-html-in-integrated-browser`, `file.open-html-in-browser` (pre-existing) | `html-preview` | native text | file tools | — |
| `.pdf` | `pdf-viewer` tab (view only) | none | `pdf-viewer` | anydoc → markdown | none (view-only) | — |
| images | `image-viewer` | none | `image-viewer` | vision path | — | — |
| code/text | `code-editor` | none | `code-editor` | native text | file tools | built-in New file |
| `.docx` | **none, deliberately** (ticket 03: native WordCraft window, not a tab) | `file.open-in-wordcraft`, `file.new-wordcraft-document` — **wordcraft, sole owner** | `code-editor` — silent binary dump (gap → ticket 17) | anydoc → markdown (unchanged) | `wordcraft-cli` MCP (live-if-open / headless) | `file.new-wordcraft-document` |
| `.xlsx`/`.csv` | univer embed tab (pending, fixed port 9123) | univer-owned ("Open in Univer"), **not yet registered anywhere in web-ui** | — (will be univer tab) | anydoc → markdown (CSV keeps source text) | univer skill (pending) | univer "New sheet" (pending) |
| `.pptx` | univer embed (pending) | univer-owned (pending) | — | anydoc → markdown | univer (pending) | — |
| `.odt`/`.rtf`/`.tex`/`.txt` | `code-editor` | **none day one** — `WORDCRAFT_DOCUMENT_EXTENSIONS = ['.docx']`; upstream's other formats stay unregistered | `code-editor` | anydoc (odt/rtf) | file tools | — |

### No-double-registration rules

1. **One entry family per format**: `.docx` → wordcraft commands only; `.xlsx`/`.csv`/`.pptx`
   → univer commands only. The overlap surface is exactly `.docx` (univer v1 Doc maps to
   docx — `univer-cli-integration/issues/06` Answer) and it resolves in WordCraft's favour
   for the **file-explorer context menu**: univer Doc stays reachable through univer's own
   embed/creation surfaces and its agent skill, but must **not** register a competing
   `Open in …` entry on `.docx`. Binding for univer's entry-points ticket (its map still
   lists "concrete entry points … where Open in Univer would appear" as fog).
2. **Entries are tool-named, so they may never be re-pointed.** "Open in WordCraft" gated on
   univer's record (or the reverse) would lie to the user. Consequence: capability gating is
   **separate per effort** — `WordcraftProbe` (services-core) gates wordcraft entries; the
   univer record (`external_integration_policy`) gates univer entries. No aggregate "docx
   edit capability" record: it would hide *which* tool is missing and break the
   verbatim-record-answer rule the skill/gate decisions already follow.
3. **No auto-pick by availability**: when WordCraft is `not-installed`, the docx entry stays
   visible and answers the wordcraft record + remediation (landed, ticket 12); it does not
   silently repoint to univer, and univer does not backfill it.
4. **Day-one format scoping holds**: WordCraft entries stay `.docx`-only even though upstream
   opens odt/rtf/html/md/tex/txt — those formats have living built-in surfaces (meditor,
   `html-preview`, `code-editor`); re-registering them would create exactly the double flow
   this ticket exists to prevent. "WordCraft owns other formats later" stays deliberate
   future scope, not drift.
5. **Creation entries namespace by tool**: `file.new-<tool>-<artifact>`
   (`file.new-wordcraft-document` landed; univer keeps its own prefix) — coexistence with
   built-in New file is fine (different artifact: blank text vs .docx), not a same-flow
   double registration.

### Shared-vs-separate gating + entry family

- **Gating: separate** (rule 2), but a **shared policy skeleton** both follow: entry visible
  on its format → capability record answered verbatim on execute (loud toast / disabled with
  reason + Recheck) → local-only in remote contexts (`localFileActionsDisabled`, registry
  `LocalOnly`/`ControllerLocal` rows — wordcraft rows landed, univer rows pending its
  effort). The two records never merge; the *pattern* is the shared contract.
- **Entry implementation family: deferred.** `FileExplorerMenuProvider` now holds three
  same-shaped blocks (HTML, wordcraft, later univer). Do not extract a shared helper yet.
  Extraction criterion: when univer's `.xlsx`/`.pptx` entries land, if the guard → push →
  `localFileActionsDisabled` → `commandExecutor` shape is identical across all three,
  extract one `office-tool-entries` helper in that change — reviewed then, not
  pre-emptively.

### Agent channel relationship (feeds univer map fog item #1)

Three lanes, no overlap: **anydoc = read lane for every document format** (Read tool,
`document-read`; converts from workspace bytes in-process — no editor install needed, works
on remote workspaces; must not be replaced by a "read via editor" path). **WordCraft MCP =
docx edit lane.** **univer = xlsx/csv/pptx edit lane.** Reading never routes through an
editor tool; editing never routes through anydoc.

### Gap uncovered → new ticket 17

Double-click / `file:open` on `.docx` still routes to `code-editor` (`getEditorType` has no
docx branch; `FilesPanel.handleOpenFile` is unconditional) — a silent binary dump, the exact
"silent viewer fallback" decision 03 forbids once WordCraft is a registered surface. Decided
routing: capability `available` → launch the WordCraft window (same action as the context
entry); otherwise the loud record state + remediation, never the code-editor dump. Carried
as ticket 17 rather than folded into 16 (implementation, not validation).

Verification for this decision: web-ui `check:web` + the `FileExplorerMenuProvider` contract
tests when 17 lands; this ticket itself changes only `.scratch/`.

### Grilling round 2 confirmation (2026-10-17)

User decisions reaffirm and sharpen this answer:

- **Ownership endgame confirmed: WordCraft gets .docx now** — when the capability record is
  `available`, default-open routes to WordCraft (ticket 17's decided routing stands).
- **Concurrent open (new rule): last write wins, accepted.** No interlock between the live
  WordCraft editor and anydoc/panel surfaces; the lost-edit risk is a documented, accepted
  trade-off. If it bites, a lock/notify follow-up gets its own ticket — do not grow a lock
  protocol silently on top of existing behavior.
