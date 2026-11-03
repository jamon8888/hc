# 20: Embedded WordCraft command bridge

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: 18 (Embedded document-control contract and artifact handshake), 19 (Human-editable embedded `.docx` surface)

## What to build

Connect Alex's embedded document resource to WordCraft's existing UI-thread control
queue so a caller can act on the same visible editor through a versioned,
correlated parent/iframe protocol.

## Acceptance criteria

- [ ] Parent requests and iframe results carry a request id, document session
      identity, expected revision, and explicit success or error state.
- [ ] Requests are dispatched through WordCraft's existing UI-aware command path,
      not by calling the document engine directly from browser code.
- [ ] The bridge supports command discovery, single commands, batches, text reads,
      document inspection, page rendering, and save requests using the established
      WordCraft command vocabulary.
- [ ] Unknown commands, invalid parameters, stale session identities, timeouts,
      and malformed responses produce explicit errors.
- [ ] A real browser probe demonstrates a command changing the visible document and
      returning a correlated result.
- [ ] Bridge tests cover origin/source validation, correlation, ordering, errors,
      and frame reload while preserving the host-save contract.
