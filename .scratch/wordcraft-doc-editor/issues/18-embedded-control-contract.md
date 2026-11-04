# 18: Embedded document-control contract and artifact handshake

`Type: task`
`Status: resolved`
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

## Comments

2026-11-03 implementation slice:

- Added the managed web-bundle capability record, pinned version/protocol/command
  manifest validation, explicit unavailable/error remediation, and tolerant
  `web` deserialization for older native capability records.
- Added the browser ready-message validator for trusted origin/source, session
  identity, malformed payloads, protocol/version checks, and required command
  capabilities. Its focused tests cover accepted and fail-closed startup states.
- Kept native CLI/app probing and headless behavior independent of web-bundle
  availability, and stabilized the `not-installed` status wire value.
- The actual embedded iframe host/presenter, artifact checksum/attribution
  verification, and production message-listener wiring are downstream consumers
  in tickets 19 and 20. This contract is resolved independently; those tickets
  may proceed against its versioned handshake without claiming this ticket.

2026-11-04 continuation:

- The managed web probe now requires and verifies `SHA256SUMS.txt` before it can
  report `available`; missing sidecars, unlisted files, mismatches, and tampering
  return an explicit error and leave `checksumVerified` false. Legacy capability
  records default that field safely. The real pinned artifact and GPU-backed
  browser probe remain release gates.
