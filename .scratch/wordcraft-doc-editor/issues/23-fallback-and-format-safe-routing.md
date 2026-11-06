# 23: Fallback and format-safe routing

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: 19 (Human-editable embedded `.docx` surface), 21 (Single-writer persistence and recovery)

## What to build

Route `.docx` opening to the embedded surface only when it is safe, available,
and enabled by the rollout decision in ticket 25. Until that gate is recorded,
the native WordCraft path remains the default. Preserve headless behavior and
ownership of every other document format.

## Acceptance criteria

- [ ] Opted-in `.docx` opening selects the embedded surface only after the shared
      capability and protocol handshake succeed; the native path remains the
      default until ticket 25 records the rollout decision.
- [ ] If embedding is unavailable but the local native WordCraft capability is
      available, Alex offers the existing native-window path with visible state.
- [ ] If neither path is available, Alex shows the capability record and
      remediation; it never opens a silent binary/code-editor dump.
- [ ] Explicit open-as-text bypasses the WordCraft route, and Markdown, HTML, PDF,
      spreadsheet, code, and other existing format owners remain unchanged.
- [ ] Persisted sessions with missing embedded fields remain readable, and an
      incompatible protocol leaves the record intact with an actionable state.
- [ ] Routing tests cover available, unavailable, incompatible, explicit-bypass,
      non-`.docx`, and upgrade cases.
