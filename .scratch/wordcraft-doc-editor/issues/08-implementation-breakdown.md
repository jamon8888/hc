# implementation-breakdown

`Type: task`
`Status: resolved`

## Question

Rewrite the destination into a concrete implementation ticket set: exact crates/apps touched,
the capability record + settings wires, new panel types and registry rows, managed MCP server
registration, and the verification commands per local AGENTS.md.

## Answer

Destination holds as redrawn; the implementation ticket set is issues **09–16**, grounded in the
spike's proven path:

- `09` wordcraft-capability-record (task, unblocked) — capability record + pinned managed
  component + `ManagedRuntimeResolver` order, services-core/services-integrations.
- `10` headless-wordcraft-session (task, blocked by 09) — the agent's primary editing path:
  per-workspace-session server lifecycle, process_manager spawns, MCP registration + tool
  guidance (including spike-caught `paragraphs:true` detail).
- `11` live-if-open-bridge (task, blocked by 10) — visible-session MCP `--connect` bridge,
  adapter-only key handling, single-writer rule.
- `12` editor-surface-entries (task, blocked by 09) — "Open in WordCraft"/"New Docx" entries,
  controller-local visible window, loud-gated states.
- `13` doc-surface-relationship (grilling, unblocked) — WordCraft vs meditor/anydoc/univer seams.
- `14` remote-provisioning-design (grilling, blocked by 09, 10) — the workspace-host component
  provisioning path that ticket 06's scope expansion obligates.
- `15` registry-and-capability-ads (task, blocked by 09) — Product Operation Registry rows /
  closure tests.
- `16` validation-loop (task, blocked by 10–12) — round-trip/e2e/dispatch tests, real-GPU
  visible-app validation, optional embed probe, durable docs.

Verification is at the owner ticket (focused crate tests, `check:core-boundaries`, `fmt:rs`);
zero new theme tokens by decision 03; this wayfinding effort is complete once the map's
Destination — a bundled, working integration with a recorded contract — is satisfied by
09–16 landing, with 13/14 sharpening as they resolve.

