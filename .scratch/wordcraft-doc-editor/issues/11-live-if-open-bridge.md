# live-if-open-bridge

`Type: task`
`Status: resolved`

## Question

Implement the live-if-open rule: when the target document is open in the visible WordCraft
window, agent tool calls bridge to *that* session (`wordcraft-cli mcp --connect 127.0.0.1:<port>`
with the key taken from the control-key file, read only by OpenBitFun's desktop adapter, never
logged, never in agent context); otherwise the headless server (ticket 10) edits the file.
Single-writer rule per document; what happens when the visible window closes mid-batch
(migrate to headless loudly, don't silently copy). Key + profile per ticket 05.

## Answer

Landed in `mcp::server::wordcraft_session` (same module as ticket 10):

- `WordcraftLiveBridgeSpec::new(session_id, control_port, control_key)` — validated live-bridge
  definition reusing the session-id rules; port must be 1..=65535; the key must be non-empty and
  control-char-free. Upstream contract honored: `--connect` carries **no** automation roots
  (upstream refuses that combination) and no profile env — the visible app's own process owns its
  profile (ticket 05).
- `build_session()` — `wordcraft-cli mcp --connect 127.0.0.1:<port>` with
  `WORDCRAFT_CONTROL_KEY` as the bridge child's env only (the adapter reads the key file; the key
  is never logged and never enters agent context — upstream forwards it to loopback exclusively,
  which is the only supported connect target here; remote goes through ticket 14's provisioning).
- `WordcraftSessionRoute` + `wordcraft_route_session(open_in_visible_app, live_bridge, headless)`
  — the pure single-writer decision:
  - not open → `Headless` (the file is unowned, headless edits it);
  - open + reachable bridge → `Live` (user watches edits; no second-writer conflict);
  - open + bridge missing → `Refused(reason)` **loud** — never a silent race of headless writes
    against a dirty or freshly-closed window; the reason instructs Recheck or save-then-close.
  Loud migration on window-close-mid-batch therefore falls out of the same decision on the next
  route call: the desktop adapter's window state flips `open_in_visible_app` to false only after
  the window is verifiably closed, and until then the file stays single-writer-refused.

Verification: 8/8 module tests (`routing_follows_live_if_open_else_headless` covers all three
routes with argv assertions), 30/30 family lib tests, `check:core-boundaries` pass.

Carried over: the *use* sites — desktop adapter producing `document_open_in_visible_app`
(window/protocol state from ticket 05) and reconciling `Refused` into a loud user/agent state —
belong to the entry-point/orchestration wiring (ticket 12's surface + core registration), not to
this crate.


## Question

Implement the live-if-open rule: when the target document is open in the visible WordCraft
window, agent tool calls bridge to *that* session (`wordcraft-cli mcp --connect 127.0.0.1:<port>`
with the key taken from the control-key file, read only by OpenBitFun's desktop adapter, never
logged, never in agent context); otherwise the headless server (ticket 10) edits the file.
Single-writer rule per document; what happens when the visible window closes mid-batch
(migrate to headless loudly, don't silently copy). Key + profile per ticket 05. Verification:
focused tests of the bridge routing + key-file handling in the desktop adapter layer.
