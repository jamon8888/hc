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

2026-11-03: Started the integration slice after ticket 18. Added the parent
`WordcraftDocumentHost` lifecycle seam, save acknowledgements, dirty/failure/
teardown handling, and workspace-owner base64 byte writes with optimistic hash
checks across desktop, server, and CLI peer adapters. The dedicated canvas
presenter and `.docx` routing remain pending because the pinned WordCraft web
capability currently exposes a filesystem artifact path, not a browser-loadable
URL or bundled iframe host.
