# 20: Embedded WordCraft command bridge

`Type: task`
`Status: claimed`
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

## Comments

2026-11-05 implementation continuation:

- The parent `WordcraftBridge` now serializes correlated commands through one
  in-flight request. This preserves WordCraft's UI-thread ordering for edits and
  the reads that verify them, rather than allowing concurrent frame messages to
  race. Queued requests are rejected together with the active request on frame
  failure or teardown, and request timeouts advance the queue safely.
- Added ordering and queued-failure regression tests. The focused WordCraft Web
  UI set passes 44/44 tests across the bridge, host persistence, session recovery,
  protocol, API, and routing suites.
- The ticket remains claimed until the packaged patched web artifact and the
  real-device visible command probe are release-validated; those gates are
  recorded in tickets 18, 19, and 25.
