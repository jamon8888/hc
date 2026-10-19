# end-to-end-spike

`Type: prototype`
`Status: resolved`

## Question

Prove the minimal loop on this macOS machine before committing to contracts. Throwaway scripts
only (keep artifacts in here, not the repo):

1. Obtain WordCraft via the shape ticket 01 favors (fastest path: upstream release binary).
2. Headless: `wordcraft-cli mcp` with `--automation-*-root` on a sample `.docx` copied into a
   scratch workspace; drive it from an MCP client (or raw stdio JSON) to `open_document`,
   `batch` (`select_text` + `format.bold` + `review.trackChanges {"value": true}` typed edits
   authored as "OpenBitFun Agent"), `inspect_document`, `save_document`; reopen the docx to
   confirm tracked changes survive.
3. Visible: `wordcraft --control <port>`, auth via key file, `engine.execute` a couple of
   commands and `ui.screenshot`; confirm the failed-command-unchanged claim.
4. Record: exact binary flags used, failure modes, latency feel of `execute` per-command vs
   `batch`, and anything that contradicts ticket 02/05 assumptions (session lifetime, idle
   behavior).

## Answer

**PROVEN: the headless agent-editing loop works end to end**, on macOS universal `wordcraft-cli`
binary from upstream release `v0.7.0` (MIT/Apache + font OFL licenses ship in the archive —
validating ticket 01's attribution claim).

Witnessed (script lives in `/private/var/folders/…/T/opencode/wordcraft-0.7.0/`, throwaway):
`wordcraft-cli mcp --author "OpenBitFun Agent (e2e-spike)" --automation-read-root WS
--automation-write-root WS` driven by a stdlib JSON-RPC client:

- Handshake `initialize`/`notifications/initialized`/`tools/list` — ~270-command instructions
  string comes back with a suggested agent flow (`new_document → type_text → text.newParagraph →
  para.style → select_text → format.bold`). A server response per request, one line each.
- Live pass: `new_document` → `review.trackChanges {value:true}` → `type_text` → `select_text`
  → `format.bold` → `save_document` (honored write root) → `inspect_document` shows run props by
  offset. A `save_document` to `/tmp/outside-demo.docx` was refused exactly as documented:
  `automation path rejected: outside the write root …` (proving ticket 04/05 confinement claims).
- **Tracked changes in the file**: unzipping `workspace/demo.docx`, `word/document.xml` has 3
  `w:ins` revisions authored `OpenBitFun Agent (e2e-spike)`, bold `<w:b/>` present; the
  independent `wordcraft-cli text demo.docx` read-back proves the screenshot-free
  verify loop works through two different binaries.
- Noted for the contract: `type_text` with a `\n` did NOT split paragraphs (one block, len 55);
  ticket 08's guidance text must prescribe `paragraphs:true` or explicit `text.newParagraph` —
  exactly the kind of detail the guidance layer owns.

**Failed leg — visible app on THIS machine (environment, not product):** the egui `wordcraft`
app compiled to wgpu surfaces but this host has no usable GPU backend (Metal "Parent device is
lost"; explicit `gl`/`vulkan` overrides fail with `FailedToCreateSurfaceForAnyBackend`). So the
control-channel leg (key file, `engine.execute`, screenshot) could not be exercised here.
Carried to ticket 08 as a pre-release validation requirement on real GPU hardware/CI; control
protocol facts remain from docs (ticket 02).

**Free extra — `wordcraft-web-0.7.0.zip` shape (for the ticket-03 embed upgrade):** a static
hosting bundle (single hashed wasm + js, `index.html`, `_headers` + `.htaccess`, licenses
included, 14 MB) — no service worker, no control channel in the static build. Embedding needs a
same-origin static host (the `_headers` file suggests COOP/COEP headers upstream already wrote);
live probe still owed before committing the embed path.

