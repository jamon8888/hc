# Embedded WordCraft document editor

`Triage label: ready-for-agent`

## Problem Statement

Alex can already open `.docx` files in a separate WordCraft desktop window and can
edit them through WordCraft's headless or live MCP channels. That does not provide
the intended Alex experience: the document is not visible in the Alex window, and
the left-side composer cannot act on the exact document surface the user is
looking at.

The current WordCraft web bundle has a useful same-origin host bridge for opening
document bytes, reporting dirty state, and returning saved bytes. It does not yet
expose a parent-to-editor command channel. A byte-only iframe combined with a
separate headless agent session would create two writers and could overwrite
unsaved edits. The feature therefore needs a single embedded document session,
an explicit control contract, and a durable save/acknowledgement path.

The existing native-window, headless, remote-workspace, peer-device, and detached-
dispatch behavior must continue to degrade loudly when the embedded surface is
unavailable. The embedded surface must not silently edit a remote document on the
controller machine or silently fall back to a second local writer.

## Solution

Add an embedded WordCraft document surface to Alex's existing session layout. The
WordCraft editor occupies the right content canvas while the existing composer
remains on the left. Opening a supported `.docx` creates one Alex-owned document
session that owns the file identity, workspace routing, dirty state, revision,
WordCraft surface, and agent-control state.

The embedded surface uses a pinned WordCraft web artifact hosted under the Alex
webview origin. Alex supplies document bytes through the existing WordCraft host
bridge and persists returned bytes through the workspace owner. A new, versioned
parent/iframe control adapter carries allowlisted document commands and correlated
results through the same WordCraft UI-thread queue used by the native control
channel. The agent and the visible editor therefore act on one `WordApp` owner.

The composer stays a generic FlowChat input. Alex attaches the active document
session as turn context; the composer does not import WordCraft types or call
iframe APIs. The existing WordCraft MCP vocabulary remains the agent-facing
contract: command discovery, single commands, batches, text reads, document
inspection, rendering, and save.

When the web artifact or embedded control contract is unavailable, Alex keeps the
existing native WordCraft window path where it is supported and reports the
capability record and remediation when it is not. Headless editing remains the
route for invisible local, remote-workspace, and detached work. No fallback may
copy a remote file to the controller and edit it locally.

## User Stories

1. As an Alex user, I want to open a `.docx` inside the Alex window, so that I do not have to switch to a separate WordCraft window.
2. As an Alex user, I want the WordCraft page canvas to occupy the right side of the session layout, so that the document and conversation are visible together.
3. As an Alex user, I want the existing composer to remain on the left, so that I can continue using the familiar Alex conversation workflow.
4. As an Alex user, I want a turn sent from the composer to include the active document context, so that the agent acts on the document I am viewing rather than an ambiguous path.
5. As an Alex user, I want agent edits to appear in the embedded editor as they are applied, so that I can observe the work without opening another application.
6. As an Alex user, I want the agent and the visible editor to share one document session, so that agent edits cannot race a second headless WordCraft writer.
7. As an Alex user, I want agent changes to remain tracked changes authored as `Alex Agent (<session>)`, so that I can review and accept or reject them in WordCraft or Microsoft Word.
8. As an Alex user, I want the agent to discover available WordCraft commands, so that prompts can use the current upstream command catalog instead of hard-coded assumptions.
9. As an Alex user, I want the agent to batch related document edits, so that a multi-step change is efficient and can be verified as one logical operation.
10. As an Alex user, I want the agent to read document text and structured document inspection data, so that it can verify the result without relying on screenshots.
11. As an Alex user, I want the agent to render a page for visual verification when needed, so that layout changes can be checked in addition to document structure.
12. As an Alex user, I want every agent edit batch to save through the embedded document owner, so that the changes are durable in the workspace file.
13. As an Alex user, I want Alex to acknowledge a successful save only after the workspace owner has persisted the returned bytes, so that a reported success cannot hide a failed write.
14. As an Alex user, I want a visible dirty indicator and unsaved-change warning, so that I understand when the document has in-memory edits that are not yet durable.
15. As an Alex user, I want save failures to be shown with an actionable error, so that I can retry or choose another location without losing the document session.
16. As an Alex user, I want an iframe reload, editor crash, or lost bridge to produce an explicit recovery state, so that Alex never claims that an unsaved document was saved.
17. As an Alex user, I want to open a new blank WordCraft document in the same embedded surface, so that I can create a document without leaving Alex.
18. As an Alex user, I want explicit “open as text” behavior to remain available, so that I can intentionally bypass the WordCraft surface when I need raw file inspection.
19. As an Alex user, I want non-WordCraft formats to keep their existing owners, so that Markdown, HTML, PDF, spreadsheets, and code do not acquire competing editor routes accidentally.
20. As an Alex user, I want the embedded editor to resize with the content canvas, so that page layout, zoom, and keyboard focus remain usable when the chat pane is resized.
21. As an Alex user, I want keyboard, paste, selection, and scrolling to work inside the embedded editor, so that the surface behaves like a real document editor rather than a static preview.
22. As an Alex user, I want the embedded editor to report whether its WebGPU/WebGL2 runtime is usable, so that unsupported devices receive a clear remediation instead of a blank panel.
23. As an Alex user, I want the native WordCraft window to remain available when embedding is unavailable, so that upgrading the surface does not remove an already-supported editing path.
24. As an Alex user, I want a capability mismatch between Alex and the WordCraft web artifact to fail closed, so that an incompatible bundle cannot corrupt document bytes.
25. As a local-workspace user, I want the embedded editor to read and save through the local workspace owner, so that Alex's normal file permissions and path rules remain authoritative.
26. As a remote-workspace user, I want document bytes and saves to stay on the workspace host, so that a remote document is never silently copied to the controller for local editing.
27. As a remote-workspace user, I want an unsupported embedded route to explain what is unavailable, so that I can choose the supported headless route or fix the host capability.
28. As a remote-control user, I want a mobile or bot surface to receive an explicit unsupported response when it cannot display the embedded editor, so that a request does not hang waiting for a desktop-only UI.
29. As a peer-device user, I want the controller to know whether embedded editing is available on the executing peer, so that commands are routed to the correct host instead of running locally by accident.
30. As a detached-dispatch user, I want a job to use the negotiated headless WordCraft capability, so that it does not depend on a browser iframe that may disappear when the controller disconnects.
31. As an Alex maintainer, I want the composer to remain independent of WordCraft implementation details, so that other document surfaces can use the same conversation seam.
32. As an Alex maintainer, I want WordCraft's engine and UI behavior to remain upstream-owned, so that Alex does not fork the document model or duplicate WordCraft commands.
33. As an Alex maintainer, I want the embedded protocol to be versioned and capability-advertised, so that WordCraft upgrades can be tested and rolled back safely.
34. As an Alex maintainer, I want the browser bridge to validate origin, source, session identity, and command allowlists, so that same-origin messaging cannot accidentally control the wrong document session.
35. As an Alex maintainer, I want the feature to preserve old persisted document-session records, so that upgrades do not delete or invalidate user work when new embedded fields are absent.
36. As an Alex maintainer, I want independent OOXML/text read-back after integration tests, so that a visually successful editor session also proves that durable document content is correct.

## Implementation Decisions

- The highest seam is one Alex-owned embedded document-control adapter. It is
  consumed by the document surface, the WordCraft session route, and the active
  composer context. The composer itself remains unchanged except for attaching
  document-session context to a turn.

- The existing WordCraft session routing remains the policy owner for agent
  execution. It gains an embedded live owner alongside the existing native live,
  headless, and refused states. Routing must never start a headless writer for a
  document currently owned by the embedded editor.

- The visible surface is a dedicated WordCraft document resource/panel in the
  right content canvas. It is not represented as a code editor, text buffer,
  binary viewer, or generic browser tab. The resource stores a stable document
  identity, workspace identity, display name, surface state, dirty state, and
  revision with tolerant defaults for older records.

- `.docx` remains owned by WordCraft. Explicit “open as text” remains an
  intentional override. Other formats retain their existing product owners and
  do not gain WordCraft entries as part of this work.

- The pinned WordCraft web artifact is an immutable managed capability, separate
  from the native WordCraft binary but version-paired with it. The capability
  record reports artifact availability, protocol version, runtime support, and
  remediation. The web bundle is accepted only after a handshake confirms the
  expected host protocol and command capabilities.

- The web artifact is hosted same-origin with Alex's webview so the existing
  WordCraft open/save bridge can function. The host validates `event.origin` and
  `event.source`, rejects unknown session identifiers, and never places
  filesystem credentials or native control keys in iframe messages.

- The parent/iframe protocol is request/response based and versioned. Its minimum
  message vocabulary is:

  | Direction | Message | Required meaning |
  | --- | --- | --- |
  | iframe → parent | `ready` | Reports WordCraft version, bridge protocol version, and supported command capabilities. |
  | parent → iframe | `open` | Supplies document name, bytes, session identity, and base revision. |
  | parent → iframe | `command` | Carries request id, session identity, expected revision, author, method, and validated parameters. |
  | iframe → parent | `result` | Returns the request id, success/error, result value, dirty state, and current revision. |
  | iframe → parent | `dirty` | Reports the current dirty state and revision transition. |
  | iframe → parent | `save` | Returns the bytes and save metadata for the parent workspace owner to persist. |
  | parent → iframe | `save-ack` | Confirms durable persistence or returns a durable-save error. |
  | both directions | `failed` | Reports startup, bridge, protocol, or document errors without fabricating success. |

- The exact WordCraft command methods are mapped through the existing upstream
  control vocabulary: command listing, `engine.execute`/`command`, batches,
  document inspection, text retrieval, page rendering, and save. Arbitrary UI
  automation such as clicks, key injection, and screenshots is not required for
  the first embedded agent contract.

- The bridge dispatches commands through WordCraft's UI-thread control queue and
  `WordApp::execute`, not by calling the document engine directly from browser
  JavaScript. This preserves UI side effects, document routing, save semantics,
  and the distinction between user commands and agent commands.

- Mutating command requests are serialized per embedded document session. A
  request with a stale session or document revision is rejected with a typed
  conflict result. The parent never resolves a save until the workspace owner
  has persisted the returned bytes. Save errors leave the document dirty and the
  session recoverable.

- Agent authorship is established from Alex's session identity and applied to
  the WordCraft session explicitly. Tracked changes are enabled with an explicit
  value on every agent batch; the implementation must not rely on a toggle whose
  meaning depends on prior document state.

- The agent receives the active document as structured session context through
  the existing conversation/turn path. Chat input does not know about iframe
  windows, WordCraft services, save bytes, or command schemas.

- The active document resource is the single writer owner for the embedded
  surface. A native WordCraft window, headless server, or second embedded frame
  cannot silently attach to the same live document. If ownership is lost, the
  agent route returns an explicit unavailable/conflict state.

- Native WordCraft remains the compatibility fallback for local surfaces when the
  embedded web capability is unavailable. This fallback is selected from the
  shared capability record and is visible to the user; there is no silent
  copy-edit-copy fallback.

- Remote-workspace document I/O is routed through the workspace host. The
  embedded controller is only a presentation/control surface. Remote POSIX paths
  remain remote strings and are never joined with controller-host path semantics.
  If the browser surface cannot reach the remote document-control owner, Alex
  returns an explicit unsupported state.

- Remote control, peer-device mode, and detached dispatch keep their existing
  ownership rules. A visible embedded frame is controller-local unless a
  negotiated host capability proves otherwise; headless remote and detached work
  continues through the host-side WordCraft session. Any new host operation is
  represented in the Product Operation Registry and capability advertisements.

- The implementation must preserve the upgrade contract: new fields are
  additive and defaulted, old names remain readable during migration, and an
  incompatible or unreadable session is retained with an actionable unsupported
  state rather than deleted.

- WordCraft owns its editor chrome, layout, and internal localization. Alex adds
  only the surrounding surface, status, error, and session-context copy through
  the existing i18n policy. No new semantic theme colors are required for the
  WordCraft canvas.

## Testing Decisions

- Tests must exercise observable behavior at the highest available seam. They
  should verify routing, message behavior, save durability, and user-visible
  degradation rather than asserting private iframe implementation details.

- Web UI contract tests will cover `.docx` resource creation, embedded-panel
  mounting, active-document context attachment, explicit text-open bypass, panel
  teardown, resize/focus behavior, and preservation of existing non-WordCraft
  format ownership.

- Browser bridge tests will use a fake iframe/parent message endpoint to cover
  handshake negotiation, origin/source rejection, session nonce validation,
  open/ready ordering, command correlation, command timeout, malformed payloads,
  dirty transitions, save acknowledgement, save failure, stale revision, and
  frame reload recovery.

- WordCraft session-route tests will cover the embedded-live, native-live,
  headless, unavailable, stale, and refused states. They must prove that a
  document owned by the embedded surface is never edited by a second headless
  process and that capability failures are returned verbatim with remediation.

- Agent-context tests will cover composer submission with an active document,
  no active document, a closed frame, a document switch during a pending turn,
  tracked-change authorship, explicit tracking enablement, batch results, and
  verify-after-batch reads.

- Save tests will prove that `wordcraft:save` bytes are written through the
  correct workspace owner, that success is acknowledged only after persistence,
  that an export does not replace the active document identity, and that a
  failed save keeps the document recoverable and dirty.

- Real WordCraft integration tests will use the pinned web artifact and a real
  `.docx`: open bytes, apply a tracked edit, save through the parent, reopen the
  persisted bytes, inspect the document, and independently verify OOXML/text
  content. The test will also exercise a representative large document.

- Native compatibility tests will keep the existing WordCraft headless round
  trip and native live-control checks as regression coverage. The embedded path
  must not weaken the existing outside-write-root refusal or authoring contract.

- Runtime tests will cover WebGPU, WebGL2 fallback, missing canvas, unsupported
  browser/runtime, resize, keyboard focus, paste, frame reload, and startup
  failure on supported desktop hardware. A local compile is not evidence that a
  real GPU surface works.

- Remote-workspace tests will run document open, command execution, save, and
  reconnect with the workspace owner remote. They must prove that no controller-
  local fallback occurs. Peer-device tests will verify capability advertisement
  and controller-local refusals. Detached-dispatch tests will verify negotiated
  headless execution without a live browser frame.

- Upgrade tests will load an old session/resource record with missing embedded
  fields, an older compatible protocol, and an incompatible protocol. Compatible
  records must round-trip; incompatible records must remain present and expose a
  clear remediation.

- Prior art for these tests includes the existing WordCraft capability and
  session-routing tests, file-open routing tests, WordCraft editor-command
  contract tests, the headless WordCraft `.docx` round-trip, remote-surface
  registry closure tests, and upstream host-save tests.

## Out of Scope

- Rewriting, vendoring, or forking the WordCraft document engine or UI crates.
- Chasing full Microsoft Word feature parity or changing WordCraft's upstream
  command semantics.
- Supporting every format WordCraft can parse; this feature is `.docx`-first and
  preserves the existing ownership of Markdown, HTML, PDF, spreadsheets, code,
  and other text formats.
- Exposing arbitrary native UI automation, screenshots, or synthetic mouse/key
  control as the first embedded agent contract.
- Making a mobile/bot client display a desktop WordCraft editor when that client
  has no supported visual surface.
- Making detached dispatch depend on an attached browser or controller iframe.
- Silent copy-edit-copy synchronization, last-write-wins between independent
  embedded/headless writers, or local editing of remote workspace content.
- Introducing a global document lock for unrelated editors; this spec only
  serializes ownership within the embedded document session.
- Replacing the existing native WordCraft window or headless session before the
  embedded capability and real-device validation gates pass.
- Adding a second global capability record that merges WordCraft with other
  office-tool integrations.

## Further Notes

- The cloned upstream investigation found that WordCraft already has the needed
  internal seam: `Services` for host save/dirty behavior, `WordApp::with_control`
  for UI-thread requests, `WordApp::execute` for agent-safe command execution,
  and a transport-independent MCP backend vocabulary. The integration should
  extend the web host layer rather than duplicate the engine.

- The current Alex WordCraft capability is based on a native binary and the
  existing native surface. The embedded artifact needs its own availability and
  protocol handshake while remaining version-paired with the native release.

- The current working-tree investigation did not execute live remote or GPU
  browser scenarios. Those remain explicit validation gates before changing the
  default `.docx` experience.

- The implementation should first land the control adapter and a throwaway
  real-browser probe, then enable the embedded surface behind the negotiated
  capability. The existing native route should remain available during rollout.
