# 21: Single-writer persistence and recovery

`Type: task`
`Status: claimed`
Triage label: ready-for-agent

Blocked by: 19 (Human-editable embedded `.docx` surface), 20 (Embedded WordCraft command bridge)

## What to build

Make the embedded resource the single owner of its live WordCraft document and make
save completion durable, revision-aware, and recoverable.

## Acceptance criteria

- [ ] The embedded session records stable document identity, workspace identity,
      surface ownership, dirty state, and a monotonic revision with tolerant
      defaults for older records.
- [ ] Mutating requests are serialized for one embedded document session and a
      stale session or revision is rejected without overwriting newer content.
- [ ] Alex acknowledges a save only after the workspace owner persists the returned
      bytes; a failed save leaves the session dirty and recoverable.
- [ ] A native WordCraft window or headless server cannot silently become a second
      writer for a document owned by the embedded surface.
- [ ] Frame reload, bridge loss, and editor crash produce an explicit recovery state
      and never fabricate a clean or saved document.
- [ ] Tests cover save acknowledgement, stale revisions, concurrent requests,
      failure recovery, and writer ownership.

## Implementation continuation (2026-11-04)

- The embedded host serializes returned save bytes, acknowledges only after the
  workspace write completes, keeps failed saves dirty, and exposes durable-save
  waiters to the command bridge.
- The session now assigns a generation to every `ready` handshake and reopens the
  active document after an iframe reload even when no explicit failure event was
  delivered. Focused bridge/session tests cover this recovery path.
- Remaining acceptance work is the packaged WebView/device crash validation and
  broader concurrency/writer-ownership coverage.

## Comments

2026-11-05 implementation continuation:

- Native recovery now explicitly disposes the embedded bridge/session and
  releases the hosted surface before launching the native WordCraft window.
  This prevents a late iframe/protocol failure from leaving two writers alive
  for the same document.
- Added panel regressions for native recovery and unavailable remediation. The
  packaged WebView/device crash and cross-process ownership gates remain open.
