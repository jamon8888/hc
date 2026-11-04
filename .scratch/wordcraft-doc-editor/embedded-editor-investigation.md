# embedded-editor-investigation

`Type: research`
`Status: resolved`
`Checked: 2026-11-03`

## Question

Is it feasible to embed WordCraft entirely inside Alex's main window, with Alex's
composer on the left acting on the opened `.docx` in the editor pane? What are the
actual Alex seams, what does the current upstream web host provide, and what design
keeps editing safe across local, remote, and upgraded installations?

## Answer

**Feasible as a phased product direction, but not by wrapping the current web
bundle in an iframe alone.** Current WordCraft web host mode can open a document
from the parent and return saved bytes plus dirty/error state. It cannot currently
receive `engine.execute`, `batch`, `inspect_document`, or other agent commands from
the parent. Therefore:

- The current native app path is the only existing path that gives Alex's agent a
  live, single-writer WordCraft session.
- A wasm iframe is viable for the visible editor surface and file persistence, but
  “composer acts directly on the open document” requires a versioned parent-control
  protocol (upstream or a product-owned adapter) in addition to the existing
  open/save bridge.
- The safest recommendation is to introduce an Alex document-session/control port,
  keep the native live bridge as the fallback, and make the wasm embed the first
  consumer only after a real control-and-save probe passes.

## Current Alex behavior and exact seams

The current layout already matches the desired visual relationship. `SessionScene`
documents the left-to-right layout as `ChatPane`, resizer, and `AuxPane`/`ContentCanvas`
([`src/web-ui/src/app/scenes/session/SessionScene.tsx:3-8`](../../src/web-ui/src/app/scenes/session/SessionScene.tsx#L3-L8)); the actual main row mounts the chat pane on the left and auxiliary pane on the right
([`src/web-ui/src/app/scenes/session/SessionScene.tsx:496-575`](../../src/web-ui/src/app/scenes/session/SessionScene.tsx#L496-L575)). `ChatPane` mounts the shared FlowChat container and `ChatInput`
([`src/web-ui/src/app/scenes/session/ChatPane.tsx:186-214`](../../src/web-ui/src/app/scenes/session/ChatPane.tsx#L186-L214)). The composer sends normalized text and context through `useMessageSender`, which ultimately calls `FlowChatManager.sendMessage`
([`src/web-ui/src/flow_chat/hooks/useMessageSender.ts:116-128`](../../src/web-ui/src/flow_chat/hooks/useMessageSender.ts#L116-L128), [`src/web-ui/src/flow_chat/hooks/useMessageSender.ts:257-287`](../../src/web-ui/src/flow_chat/hooks/useMessageSender.ts#L257-L287)). This is the right product seam for attaching an active document context; the composer itself should not know how WordCraft edits.

The existing WordCraft surface is not an in-window panel:

| Concern | Current seam | Consequence for embedding |
| --- | --- | --- |
| Capability | `WordcraftCapability` reports CLI/app resolution and status, but no web artifact ([`src/crates/services/services-core/src/wordcraft.rs:48-63`](../../src/crates/services/services-core/src/wordcraft.rs#L48-L63)) | Add a web-bundle capability/version or do not claim the embed is available. |
| Native launch | `wordcraft_open_document_command` resolves local `.docx`, refuses remote targets, reserves a loopback port, creates an isolated profile, and spawns the app ([`src/apps/desktop/src/api/wordcraft_api.rs:230-289`](../../src/apps/desktop/src/api/wordcraft_api.rs#L230-L289)) | This is a separate OS window, not a panel. |
| Default `.docx` open | `FileTabManager` routes `.docx` to the capability-aware native launcher and refuses a binary-dump fallback when the capability is known ([`src/web-ui/src/shared/services/FileTabManager.ts:91-105`](../../src/web-ui/src/shared/services/FileTabManager.ts#L91-L105), [`src/web-ui/src/shared/services/FileTabManager.ts:211-255`](../../src/web-ui/src/shared/services/FileTabManager.ts#L211-L255)) | Replace or branch this decision to create an embedded document resource. |
| Panel model | `PanelContentType` has no WordCraft type ([`src/web-ui/src/shared/types/panelContent.ts:1-34`](../../src/web-ui/src/shared/types/panelContent.ts#L1-L34)); `FlexiblePanel` has no WordCraft renderer ([`src/web-ui/src/app/components/panels/base/FlexiblePanel.tsx:298-454`](../../src/web-ui/src/app/components/panels/base/FlexiblePanel.tsx#L298-L454)) | Add a dedicated `wordcraft-document` content type and renderer; do not reuse `code-editor`. |
| File resource identity | The file-resource set excludes WordCraft ([`src/web-ui/src/app/workbench/contentResourceStore.ts:9-14`](../../src/web-ui/src/app/workbench/contentResourceStore.ts#L9-L14)) | Add document identity/dirty/save ownership deliberately, not as a text buffer. |
| Agent route | The session router chooses live bridge, headless, or loud refusal ([`src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs:268-305`](../../src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs#L268-L305)) | Preserve this policy behind a surface-neutral document-control port. |

The native agent path is already close to the desired behavior. Headless sessions
run `wordcraft-cli mcp` with the workspace as both automation roots, an isolated
profile, and an `Alex Agent (<session>)` author
([`src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs:135-163`](../../src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs#L135-L163)). When the visible app is open, the live bridge runs `mcp --connect 127.0.0.1:<port>` with the control key and no automation roots
([`src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs:174-243`](../../src/crates/services/services-integrations/src/mcp/server/wordcraft_session.rs#L174-L243)). If the window is open but its bridge is unavailable, Alex refuses to race a second writer. This is the behavior the embedded surface should retain.

The current pinned version is `0.7.0`
([`src/crates/services/services-core/src/wordcraft.rs:272-274`](../../src/crates/services/services-core/src/wordcraft.rs#L272-L274)), while [upstream's current release page](https://github.com/storytold/wordcraft/releases) shows `0.15.0` and a separately packaged web archive. That gap makes a web-bundle/version handshake and an explicit upgrade plan mandatory before embedding.

## Current upstream web host/iframe protocol and limitations

Upstream's browser build is the same Rust/egui WordCraft front end running through
eframe's web runner, using WebGPU with WebGL2 fallback. The web entry point explicitly
states that it has **no TCP control channel**; open and save are browser services,
and `?host=parent` enables a same-origin parent-page bridge
([upstream `apps/wordcraft-web/src/main.rs`](https://github.com/storytold/wordcraft/blob/main/apps/wordcraft-web/src/main.rs)). The architecture separates the reusable engine, MCP server, egui UI, desktop app, CLI, and browser app
([upstream `docs/architecture.md`](https://github.com/storytold/wordcraft/blob/main/docs/architecture.md)).

Current `?host=parent` messages, from the upstream source, are:

| Direction | Message | Meaning |
| --- | --- | --- |
| iframe → parent | `wordcraft:ready` `{version}` | Web app is running and ready for a file. |
| parent → iframe | `wordcraft:open` `{name, bytes}` | Queue an `ArrayBuffer`/`Uint8Array` as if opened or dropped. |
| iframe → parent | `wordcraft:save` `{name, saveAs, bytes}` | Parent must persist every save/export payload. |
| iframe → parent | `wordcraft:dirty` `{dirty}` | Unsaved state changed. |
| iframe → parent | `wordcraft:failed` `{error}` | Startup failed, for example no WebGPU/WebGL2. |

These messages and the `saveAs` semantics are documented in the source comments and
implementation ([upstream `apps/wordcraft-web/src/host.rs`](https://raw.githubusercontent.com/storytold/wordcraft/main/apps/wordcraft-web/src/host.rs)). The bridge checks that the parent is on the **same origin**, rejects an opaque `file://` origin, validates the message source, and posts saves with a transferred `ArrayBuffer`. The web shell connects this bridge to WordCraft's file inbox and save/dirty services, but exposes no parent-to-engine command request path ([upstream `apps/wordcraft-web/src/web.rs`](https://raw.githubusercontent.com/storytold/wordcraft/main/apps/wordcraft-web/src/web.rs)).

## Clone-based findings: the smallest useful upstream seam

I cloned upstream `main` at commit `ffb155a` (package version `0.15.0`) and traced the implementation rather than relying only on the published web-host documentation. The clone remains outside the Alex repository; no WordCraft source is vendored. A local prototype was applied in `/tmp/wordcraft-upstream.PJcNnc` to validate the seam without pushing to the upstream project.

The important result is that WordCraft already has a narrow command seam suitable for a web host extension:

- `wordcraft-engine::Session::run` is the engine command entry point, and `wordcraft-ui-egui::WordApp::execute` is the UI-aware entry point used by scripts, MCP, and the native control channel. `execute` deliberately avoids interactive save prompts, while `WordApp::run` is the user-command path that may open dialogs.
- `WordApp::new` accepts a host-provided `Services` value. The existing `Services::host_save`, `Services::download`, and `Services::on_dirty` callbacks are already the intended file/dirty integration seam, and upstream has focused tests for host-save behavior across Save, Save As, exports, legacy Word input, and save failures.
- `WordApp::with_control` accepts a `ControlRequest` receiver. Each browser frame calls `WordApp::logic`, which drains that queue and dispatches through the same `control::handle` path as the native control server. The native handler calls `WordApp::execute` for `engine.execute`/`command`, and supports document inspection, command listing, batches, renders, and UI inspection around that path.
- `wordcraft-mcp` is transport-independent at its core: its `Backend` trait is `call(method, params)`, with `Headless` and TCP `Remote` implementations. The MCP tool catalog already defines `execute`, `batch`, `get_text`, `inspect_document`, `render_page`, `save_document`, and related operations. The browser cannot use the stdio/TCP transport directly, but Alex can map the same method vocabulary to a parent/iframe request-response adapter.

This means a full fork of the editing engine is not necessary. The local prototype is limited to the web shell/host layer plus the existing control request type: it creates a `ControlRequest` channel, attaches it with `WordApp::with_control`, accepts validated `command` messages from the parent, maps the shared tool vocabulary onto the UI-aware handler, and returns correlated `result` messages from the frame loop. It also carries the request author and parent save acknowledgements. Native and `wasm32-unknown-unknown` `cargo check -p wordcraft-web` both pass for the prototype; a real browser mutation/save probe is still outstanding. The prebuilt upstream web archive cannot provide this behavior by configuration alone; Alex would need either an upstream release containing the bridge, a maintained Alex web build from an upstream commit, or a fork. The preferred order is upstream contribution, then a narrowly pinned fork; vendoring the whole workspace into Alex would violate Alex's adapter boundary and create an expensive upgrade surface.

The web bridge should not call `Session::run` directly from JavaScript. That would bypass UI-side effects and document routing. It should dispatch through the control handler/`WordApp::execute`, use request IDs, include the active document/revision, and return explicit errors. `file.save` must complete only after the parent has persisted the `wordcraft:save` bytes and acknowledged that persistence. A dirty notification alone is not enough for agent control.

The clone also makes the concurrency rule concrete: the embedded bridge and Alex's agent must share one `WordApp` owner. The existing native control channel queues requests onto the UI thread and serializes them per frame; an embedded bridge should reuse that shape rather than starting a second headless session. The current host bridge has no command authentication beyond same-origin/source checks, so Alex should add a per-embed capability/session nonce and expected document revision before exposing mutating commands.

Follow-up verification on 2026-11-04: the local upstream prototype's focused
`wordcraft-ui-egui` control tests passed 22/22, and both native and
`wasm32-unknown-unknown` `cargo check -p wordcraft-web` passed. This validates
the UI-aware dispatch seam and its target builds, but it is still not a real
browser mutation/save probe and the patched web artifact is not yet packaged
into Alex's managed runtime. Those two facts remain explicit release gates.

Relevant upstream implementation points are [`Services`](https://github.com/storytold/wordcraft/blob/ffb155a/crates/ui-egui/src/services.rs), [`WordApp::execute`](https://github.com/storytold/wordcraft/blob/ffb155a/crates/ui-egui/src/app/commands.rs), [`WordApp::with_control`](https://github.com/storytold/wordcraft/blob/ffb155a/crates/ui-egui/src/app.rs), the [`control` request handler](https://github.com/storytold/wordcraft/blob/ffb155a/crates/ui-egui/src/control/handler.rs), and the [`wordcraft-mcp` backend](https://github.com/storytold/wordcraft/blob/ffb155a/crates/mcp/src/lib.rs).

Generic iframe hosting is otherwise straightforward: the release bundle is a static
site with roughly 13 MB wasm (about 5 MB compressed), needs correct wasm MIME type,
and uses WebGPU/WebGL2. The upstream hosting guide says generic cross-origin embeds
work, but the **parent host-mode protocol is same-origin only**. Sandboxed frames need
`allow-scripts allow-same-origin allow-downloads allow-popups`; `allow-downloads` is
irrelevant only if Alex handles all saves through host mode
([upstream web hosting guide](https://raw.githubusercontent.com/storytold/wordcraft/main/packaging/web/README.md)).

Important limitations for Alex:

1. `wordcraft:open` and `wordcraft:save` move whole document bytes. There is no
   incremental operation, document revision, selection/context report, command
   response, `inspect_document`, or save acknowledgement in the current bridge.
2. `?author=` sets the WordCraft session author for comments and tracked changes,
   but it is a URL input, not a per-command authenticated Alex identity.
3. A `wordcraft:dirty` notification says only dirty/clean. It does not establish a
   write lease or tell Alex what in-memory revision is being edited.
4. The source-defined `version` in `ready` is not a negotiated protocol version.
   Alex must pin the bundle and reject or fall back when the expected bridge shape
   is absent.
5. The web build is not the native control protocol. The native `engine.execute`,
   screenshots, keyboard/mouse controls, and MCP `--connect` path remain desktop
   capabilities ([upstream control protocol](https://github.com/storytold/wordcraft/blob/main/docs/control-protocol.md), [upstream MCP docs](https://github.com/storytold/wordcraft/blob/main/docs/mcp.md)).

## Native-window versus wasm-embed

| Option | Strengths | Blocking risks |
| --- | --- | --- |
| Native `wordcraft` window + `wordcraft-cli mcp --connect` | Existing Alex code; full upstream UI; live agent commands and verification; no byte rehydration loop; single-writer route already specified. | Not inside Alex's main window; current Tauri command is local-only and controller-local; requires native app/GPU and a loopback key. |
| `wordcraft-web` iframe with current parent bridge | Visually inside the existing right pane; no native child window; static artifact; parent owns remote/local file IO; dirty/save lifecycle is explicit. | No agent command channel; whole-byte synchronization; same-origin host requirement; browser/WebGPU/WebGL2 and iframe focus behavior; no direct read/inspect path. |
| In-process Rust/WASM engine integration | Could provide the strongest direct control and avoid iframe messaging. | Requires vendoring or exposing upstream engine/UI APIs, importing a large dependency surface, owning wasm ABI/lifecycle, and creating a new integration boundary; not justified before the web host contract is proven. |

The native option should remain the fallback and the first control reference. The wasm
option is the right user experience for “composer left, document right”, but only if
the control contract is upgraded. A byte-only iframe plus a headless file editor is
not a direct live editor: it creates an overwrite/conflict window whenever the iframe
is dirty.

## How composer-to-document control could work

The product boundary should be an Alex-owned `DocumentControlPort`, not a WordCraft
implementation leaked into `ChatInput`:

1. Opening `.docx` creates an active document session with workspace identity,
   document path/URI, WordCraft version, surface (`native` or `embedded`), dirty
   state, and a monotonically increasing document revision.
2. `ChatInput` keeps its current submission path. The active document context is
   attached to the agent turn by session/workspace metadata; the composer remains a
   normal prompt surface.
3. The agent's WordCraft tools resolve through the port. For the native surface,
   the port delegates to the existing live MCP bridge. For the embedded surface,
   it needs a parent-to-iframe request/response protocol that can at least carry:
   `list_commands`, `execute`, `batch`, `get_text`, `inspect_document`, and
   `save_document`, with request id, document id, expected revision, author, and
   an explicit save result.
4. The agent sets tracked changes explicitly, batches edits, saves, then verifies
   with `inspect_document`/text read-back. The port rejects a stale revision rather
   than overwriting a dirty iframe. If the embedded frame disappears or loses its
   control contract, the turn fails loudly; it does not silently switch to a
   headless writer.
5. The parent handles `wordcraft:save` by writing through the workspace owner
   (local or remote), then returns durable-save acknowledgement through the control
   protocol. A save notification without acknowledgement is insufficient for
   detached work or crash recovery.

If upstream does not add the command channel, the fallback design is an explicit
“agent edit transaction”: export the current iframe bytes, edit a temporary copy
  headlessly, compare the base revision, and ask the user to apply/reload the result.
That is safe but not the requested direct experience, so it should not be presented
as equivalent.

## Synchronization, remote, and upgrade risks

- **Unsaved user edits:** current host mode reports dirty but provides no lock. Alex
  must serialize agent operations behind a document lease: either save before the
  agent turn, or refuse the turn while dirty. Last-write-wins is unacceptable for
  a live embedded document because the parent only has whole-file snapshots.
- **Two writers:** never run headless MCP against a document owned by the iframe.
  Preserve the existing native `Live`/`Headless`/`Refused` policy and make the
  embedded adapter another live owner.
- **Remote workspace:** the iframe is a controller surface, but the bytes belong to
  the workspace host. Parent save/open must use the remote filesystem transport;
  local temporary copies must not become a silent fallback. Current Alex registry
  rows mark both WordCraft Tauri commands `LocalOnly` and `ControllerLocal`
  ([`src/crates/contracts/product-domains/src/remote_surface/table.rs:770-771`](../../src/crates/contracts/product-domains/src/remote_surface/table.rs#L770-L771)). An embedded surface needs new registry/capability semantics: controller-local UI plus a routed document byte/control port, or an explicit unsupported state.
- **Remote control and detached dispatch:** mobile/peer controllers cannot assume
  they can reach a controller-local iframe. Detached jobs need the target's
  negotiated headless WordCraft capability and must not depend on an attached
  browser frame.
- **Origin and security:** same-origin mode is required for the upstream parent
  bridge. Host the pinned bundle under the Alex webview origin, validate
  `event.origin` and `event.source`, and never put filesystem credentials or control
  secrets in iframe messages. A generic cross-origin iframe is not a substitute for
  host mode.
- **Upgrade drift:** the local Alex pin is `0.7.0`, while upstream is already at a
  later release. Pin the web and CLI artifacts together, verify checksums, handshake
  the bridge version, and keep a native fallback until the new artifact passes the
  contract probe. Persisted Alex document-session records must tolerate missing
  fields and preserve an unreadable/unsupported record rather than deleting it.
- **Web runtime:** the web host needs a real browser canvas and WebGPU/WebGL2. Test
  WebGPU, WebGL2 fallback, resize, keyboard focus, paste, save cancellation, frame
  reload, and large `.docx` byte transfers on each supported desktop OS.

## Recommended phased design and verification plan

### Phase 0 — prove the current upstream host contract

Build the pinned web archive and mount it at the same origin as a throwaway Alex host
page. Prove `ready → open → dirty → save → failed`, checksum the returned bytes, and
verify that an iframe reload or Alex shutdown cannot silently discard dirty data.
Probe origin mismatch, opaque origin, sandbox flags, WebGPU failure, WebGL2 fallback,
focus/keyboard, resize, and a representative real `.docx`. This is a research spike,
not a product surface.

### Phase 1 — define the Alex document-session port and retain native fallback

Extract the existing live-if-open policy behind a surface-neutral document owner.
Add document identity, revision, dirty state, save acknowledgement, capability
advertisement, and a surface-specific remediation. Keep native WordCraft as the
fallback while the embed is unavailable. Add focused tests for stale revision,
window/frame disappearance, save failure, and no silent local fallback.

### Phase 2 — embed the visible editor, still without claiming agent control

Add a dedicated `wordcraft-document` panel/resource renderer in the right AuxPane.
The parent owns file IO and feeds `.docx` bytes through `wordcraft:open`; WordCraft
returns saves through `wordcraft:save`. Route `.docx` default-open to this panel only
when the web capability record and host probe are available; otherwise use the
existing native path or loud remediation. Do not register the same `.docx` flow in
the code editor.

### Phase 3 — add live embedded agent control

Prefer an upstream versioned parent-control protocol with request/response and save
acknowledgement. Require command batching, `inspect_document`, explicit author and
tracked-change state, expected revision, and an authenticated parent origin. If that
protocol cannot land upstream, keep agent edits on the native/headless path and label
the iframe as human-only; do not simulate direct control with file polling.

### Phase 4 — remote and upgrade closure

Run the embedded surface against local and remote workspace owners, peer-device
controllers, reconnect/replay, and detached dispatch. Provision/resolve web and CLI
artifacts per host as appropriate, negotiate capabilities, and test an old session
record against the new build. Only then change the default `.docx` route from the
native window to the embedded panel.

### Evidence gates

- Local Alex: focused web tests for panel/resource identity, `FileTabManager` routing,
  dirty/save state, and composer-to-document metadata; focused Rust tests for the
  document-control route; `check:core-boundaries`, formatting, and the nearest web
  check.
- Upstream artifact: a pinned-release probe covering the exact parent protocol,
  `--version`/bundle version, checksum, and a real `.docx` open/edit/save round trip.
- Agent loop: composer prompt → WordCraft command batch → tracked-change author →
  save acknowledgement → `inspect_document` and independent OOXML/text read-back.
- Concurrency: user edit while dirty, agent edit while dirty, frame reload, native
  fallback, and stale revision must all produce deterministic save/refusal outcomes.
- Remote: remote workspace byte IO, peer/controller capability advertisement, and
  detached headless editing must be exercised separately; local-only tests are not
  evidence of remote behavior.

`Conclusion: pursue the embed, but make the command/control protocol and document
ownership port explicit gates. The current iframe bridge proves a good file-hosting
seam, not yet a direct composer-to-document editing seam.`
