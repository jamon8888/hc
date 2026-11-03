# 18: Embedded document-control contract and artifact handshake

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: None (can start immediately)

## What to build

Make the pinned WordCraft web artifact a factual Alex capability and establish the
versioned handshake that must succeed before Alex presents it as an embedded
document surface.

## Acceptance criteria

- [ ] The web artifact is pinned, version-paired with the native WordCraft release,
      and verified according to the existing managed-runtime and attribution rules.
- [ ] The handshake reports WordCraft version, bridge protocol version, and the
      supported embedded command capabilities.
- [ ] A compatible artifact reports `available`; missing, unsupported, failed, or
      incompatible artifacts return a clear capability state and remediation.
- [ ] Origin, source, session identity, malformed-message, and protocol-version
      failures fail closed without claiming that the editor is ready.
- [ ] A focused probe demonstrates ready and failed startup states without
      requiring the full embedded editor rollout.
- [ ] Existing native WordCraft capability and headless MCP behavior are unchanged.
