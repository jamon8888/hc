# 24: Remote, peer-device, and detached-dispatch closure

`Type: task`
`Status: claimed`
Triage label: ready-for-agent

Blocked by: 21 (Single-writer persistence and recovery), 22 (Composer-driven live agent editing), 23 (Fallback and format-safe routing)

## What to build

Close the embedded surface against Alex's four remote scenarios without moving
remote document content to the controller or making a browser frame a hidden
runtime dependency.

## Acceptance criteria

- [ ] Remote-workspace open, command, and save operations execute through the
      workspace owner; controller-local fallback is impossible and remote POSIX
      paths remain remote strings.
- [ ] When the remote host cannot expose the embedded surface, Alex returns a loud
      unsupported state and preserves the supported host-side headless route.
- [ ] Peer Device Mode advertises the embedded capability from the Product Operation
      Registry and refuses controller-local operations on the wrong host.
- [ ] Mobile/bot remote control receives an explicit unsupported response when no
      visible embedded surface exists.
- [ ] Detached dispatch negotiates the headless WordCraft capability and does not
      depend on an attached browser frame or live controller connection.
- [ ] Remote, reconnect, capability-advertisement, and detached tests prove that no
      local content fallback occurs.

## Comments

2026-11-05: Implemented the first owner-side transport slice. MCP now accepts the
existing workspace-owner full-duplex stdio transport, drains owner stderr, and
tears down the owner process through its control handle. `WordcraftSessionSpec`
builds a shell-quoted POSIX remote command, and `WordcraftRemoteMcpSession`
keeps the remote process and MCP connection together. Focused verification:
`cargo test -p alex-services-integrations --no-default-features --features
mcp,remote-ssh-concrete --lib mcp::server::` — 33 passed; Core feature check
with `agent-runtime,git,mcp-runtime,ssh-remote,tools-wordcraft` — passed.

The host-managed pinned-artifact provisioning, persistent per-user/per-workspace
daemon attach/reconnect, peer/mobile/detached capability negotiation, and real
remote-host evidence remain required before this issue can be resolved.

2026-11-05: Review follow-up now fails closed when a persisted document tab has
no surface identity; it does not default to the local embedded editor. The
remote owner seam also requires an owner-advertised absolute POSIX executable,
version, and MCP capability before constructing a command. Focused verification:
35 MCP/server tests passed and the WordCraft panel test passed (3 tests).

2026-11-05: Added `WordcraftRemoteSessionRegistry`, which retains one
connection/workspace/session owner, revalidates executable/version/root identity
before attach, refuses mismatched keys, and has no idle reaper. The registry is
the in-process owner seam; host-managed pinned-artifact provisioning, a
cross-restart daemon, peer/mobile/detached capability negotiation, and real
remote-host evidence remain required before this issue can be resolved.
