# Map: univer-cli integration

`Status: open`

## Destination

Bundled, working integration: univer-cli — **Apache-licensed parts only, no `@univerjs-pro`** — ships inside Alex's codebase and builds together with a managed Node 24 runtime, delivering document view/edit/create end to end, plus the architecture contract recording how it works (capability gate, formats, surfaces, daemon, remote stance). Execution is on this map; the earlier "contract only, no implementation" stance was superseded when the destination was redrawn.

## Notes

- Domain: office document viewing/editing/creation (Sheet/Doc/Slide/Base/Board), agent skills, Web UI surface, process topology.
- Every session: read the root `AGENTS.md` remote-scenarios, upgrade-compat, and non-interactive child-process rules before resolving anything touching process spawning or surfaces.
- Skills: `/grilling` + `/domain-modeling` for decision tickets; `/research` (subagent) for research tickets.
- Standing preference: loud degradation over silent fallback; no repo-wide baseline changes (Node stays >=22.12 in this contract).
- Execution is in-scope for this effort (destination redrawn). The licensing constraint from ticket 02 is now a hard requirement: Apache-only, no `@univerjs-pro` packages, no rebranding of upstream code we do not hold rights to.
- Tracker: local markdown (`.scratch/`), per `docs/agents/issue-tracker.md`.

## Decisions so far

- [node24-capability-gate](issues/03-node24-capability-gate.md) — one shared Rust capability record (policy rides `external_integration_policy`), on-demand cached detection via `ManagedRuntimeResolver`, gated-visible UI entries, skill answers verbatim from the record; no provisioning.
- [embed-viewer-feasibility](issues/01-embed-viewer-feasibility.md) — CAN_EMBED: no framing-blocking headers, fixed port 9123, `?mode=embedded` URL contract; official path is same-origin proxy + Pro cowork; Tauri check deferred to ticket 08.
- [univer-licensing](issues/02-univer-licensing.md) — SAFE_WITH_DISCLOSURE: npx-at-runtime unmodified is fine; bundling/rebranding is not (Pro packages unlicensed, credential expires 2026-11-15); network surfaces must be disclosed.
<!-- one line per resolved ticket: gist + link -->

## Not yet specified

- How univer inspection/editing relates to the existing anydoc Markdown conversion in the agent Read tool (overlap vs complement).
- Concrete entry points in file/creation flows where "Open in Univer" / "New sheet/doc" would appear.
- Peer Host / Detached Dispatch stance details beyond the local-only decision (what the registry rows / capability ads say).
- What the remote-forward path would concretely require (daemon on workspace host, viewer relay) — recorded as requirements only when a format/surface ticket sharpens it.

## Out of scope

- Mobile web / IM bot viewing of Univer documents (decision: explicit unsupported; ruled out until the relay effort reopens it).
- Distributing the univer daemon to remote workspace hosts.
- Upgrading the repo-wide dev/CI Node baseline to 24 (the shipped *managed runtime* Node 24 is in scope as ticket 10; engines/CI stay where they are).
