# editor-ui-surface

`Type: grilling`
`Status: resolved`

## Answer

**NATIVE APP WINDOW day one.** OpenBitFun spawns upstream's egui `wordcraft` binary as its own
OS window for the opened document and drives/observes it over the loopback control protocol
(live agent sessions), while the headless `wordcraft-cli mcp` server covers agent work that
needs no visible app. The `wordcraft-web` wasm embed stays the *upgrade goal* gated on ticket
07's probe; viewer-only page renders are not a day-one requirement — availability problems
degrade to the loud unsupported state (gated entry + remediation + Recheck), per the univer
ticket-03 pattern, not to a silent preview.

- **Entry points**: "Open in WordCraft" on `.docx` (plus `.odt`/`.rtf` context later) in the
  file tree, and a "New Docx document" creation entry. Both visible but gated on the shared
  capability record from ticket 01's resolver decision.
- **Theme**: none added. WordCraft owns its chrome (`ui.theme system`), so OpenBitFun adds zero
  color tokens — no theme-ratchet exposure. Allowed glue: optionally forwarding an explicit
  light/dark sync later, recorded as non-required.
- **i18n**: WordCraft ships its own UI languages and follows the system language by default;
  OpenBitFun i18n catalogs are untouched.
- **Panel routing**: no new `panelContent` type day one (separate window). If the embed upgrade
  lands later, that adds its own panel-type + startup-contract work.
- Open questions handed to the owning tickets: single-document-per-process and multi-window
  behavior of the visible app (→ ticket 05 topology, exercised in 07); what the user sees in
  chat when the agent edits the live document (→ ticket 04).


Decide the day-one surface and the loud-unsupported state for browsers/platforms without it, the
panel routing (new `panelContent` type vs reusing existing editor types), theme key story (the
theme audit ratchet applies to anything visual we add), and i18n stance (upstream ships its own
UI languages).
