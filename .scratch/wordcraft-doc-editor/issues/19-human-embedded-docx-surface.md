# 19: Human-editable embedded `.docx` surface

`Type: task`
`Status: claimed`
Triage label: ready-for-agent

Blocked by: 18 (Embedded document-control contract and artifact handshake)

## What to build

Let a local user open a `.docx` inside Alex's main session layout, with WordCraft
visible in the right content canvas and the existing composer visible on the left.
The user must be able to edit and save the document before agent control exists.

## Acceptance criteria

- [ ] Opening a supported local `.docx` creates a dedicated WordCraft document
      resource in the right content canvas rather than a code editor or binary tab.
- [ ] The existing composer remains usable on the left and the editor resizes with
      the content canvas.
- [ ] The parent sends document bytes through the WordCraft host bridge and saves
      returned bytes through the local workspace owner.
- [ ] Dirty state, save failure, startup failure, keyboard focus, scrolling, paste,
      and frame teardown are visible and recoverable.
- [ ] A new blank WordCraft document can be opened in the same surface.
- [ ] Explicit open-as-text behavior and existing ownership of non-`.docx` formats
      remain unchanged.
- [ ] Focused web tests cover local open, save, dirty state, resize, teardown, and
      explicit text bypass.

## Comments

2026-11-06 continuation:

- The file-explorer "new WordCraft document" action now creates a zero-byte
  `.docx` through the workspace owner and routes it through the normal
  capability-aware file opener, so the embedded canvas is selected when the
  local web surface is available rather than opening only the native start
  screen.
- Focused command/resource tests cover the blank-document route. Packaged
  WebView and device validation remain release gates.

2026-11-03: Started the integration slice after ticket 18. Added the parent
`WordcraftDocumentHost` lifecycle seam, save acknowledgements, dirty/failure/
teardown handling, and workspace-owner base64 byte writes with optimistic hash
checks across desktop, server, and CLI peer adapters. The dedicated canvas
presenter and `.docx` routing remain pending because the pinned WordCraft web
capability currently exposes a filesystem artifact path, not a browser-loadable
URL or bundled iframe host.

2026-11-03: Added the parent `open` message and a
`WordcraftEmbeddedDocumentSession` coordinator. The coordinator opens bytes
after a validated ready handshake, reopens after frame failure/reload, forwards
dirty/save lifecycle events to `WordcraftDocumentHost`, and disposes both
bridge and host together. The local desktop now hosts the validated web bundle
behind a per-session loopback token, and `.docx` routing opens the dedicated
canvas panel when the web capability is available. A real pinned WordCraft web
artifact and browser probe are still required before this can be marked
resolved.
