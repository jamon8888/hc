# agent-interaction-contract

`Type: grilling`
`Status: resolved`

## Answer

The user-prompt → document loop, as four decisions:

1. **Tool exposure — managed MCP server.** OpenBitFun registers `wordcraft-cli mcp` as a managed
   MCP server scoped to the workspace, with `--automation-read-root`/`--automation-write-root`
   mapped onto the active workspace. The agent uses upstream's native tool surface
   (`list_commands`, `execute`, `batch`, `type_text`, `select_text`, `get_text`,
   `inspect_document`, `save_document`); OpenBitFun contributes guidance text only (the
   tracked-changes policy, verify-after-batch habit, explicit-`value` toggles). When the
   capability is unavailable, the agent's first step returns the shared capability record state
   verbatim (same relay rule as the univer ticket-03 gate — no silent fallback, no hidden skills).
   Documented limits of the roots (pathless commands still mutate the open doc; `get_text`
   returns full text) go into tool guidance, not hidden.

2. **Edits — tracked changes always on.** Every agent edit lands as a tracked change authored
   `OpenBitFun Agent (<session id>)` (`--author` at server start; live sessions set
   `file.setAuthor` explicitly). `review.trackChanges {"value": true}` is asserted per batch,
   never bare-toggled. The user reviews/accepts/rejects in WordCraft or any Word.

3. **Target — live-if-open, else headless.** When the document is open in the visible WordCraft
   window, agent tool calls bridge to *that* session (`--connect` via the MCP bridge): the user
   watches edits happen live and there is no second-writer conflict with unsaved user edits.
   Otherwise (or for dispatch/remote work) the headless server edits the file directly. The
   "always headless" model was rejected because a dirty, open window would fight headless writes
   to the same file with real user-edit loss risk.

4. **Save — the agent saves.** After each edit batch the agent calls `save_document`, so tracked
   changes are durable in the file (this is what makes review work in Word, offline copies, and
   detached dispatch jobs). Rejection of agent edits happens inside the document (reject + save),
   not by OpenBitFun reverting files.

Residual risks handed to spike (07): MCP-bridge `--connect` reachability from OpenBitFun's MCP
stack, author-shape consistency across headless/live sessions, and latency feel of
`execute`-per-command vs `batch`.

