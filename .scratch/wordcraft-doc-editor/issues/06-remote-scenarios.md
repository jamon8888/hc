# remote-scenarios

`Type: grilling`
`Status: resolved`

## Answer

Declare-or-loud stance for all four scenarios, with one deliberate scope expansion:

1. **Remote workspace — SUPPORTED (headless on workspace host; scope expansion).** The user chose
   to raise this from "loud unsupported day one" to a supported path: for a docx living in a
   remote workspace, the headless `wordcraft-cli mcp` server runs **on the workspace host**, with
   the managed component provisioned there (same pinned tag + `SHA256SUMS.txt` verification, per
   ticket 01) over the workspace transport. Never copy-edit-copy locally, never silent local
   fallback — if the host cannot run the binary (arch/OS/offline), the state is loud. POSIX-path
   rules apply everywhere (no host `std::path` splitting). NEW OBLIGATION: the concrete
   provisioning design (transport, verification, resolution on the host) is a required piece of
   ticket 08's implementation breakdown — the univer effort deferred exactly this ("daemon to
   remote hosts"), wordcraft's permissive license makes it feasible but the work is real.
2. **Remote control (mobile web / IM bots) — agent edits yes; the window no.** A bot-driven
   session may ask the agent to edit a document (headless path, per 04/05), but the *visible
   WordCraft window* and its live control channel (`ui.click`, `ui.key`, screenshots) are a local
   desktop surface only — the protocol has no second-driver multiplexing, so remote surfaces get
   the explicit unsupported reply, not an attempt.
3. **Peer Device Mode — register per root rules.** Any new surface commands get Product
   Operation Registry rows (no hand-written tables; rows declare remote-workspace stance and
   `ControllerLocal` flags): visible-window launch answers `ControllerLocal`; headless agent
   editing proxied; capability ads derive from the registry.
4. **Detached dispatch — SUPPORTED headless.** A dispatch job on a headless target edits a docx
   through the headless MCP server, tracked changes + `Alex Agent` authorship + agent
   saves included — the exact case ticket 04's contract and ticket 05's lifecycle already cover.
   Required target capabilities (managed wordcraft component present or provisionable) must be
   advertised/negotiated, never assumed.

Guardrail carried to ticket 08: state which scenarios each change was exercised in; local-only
tests are not evidence of remote behavior.

