# wordcraft-capability-record

`Type: task`
`Status: resolved`

## Question

Wire the shared capability record for WordCraft following the univer ticket-03 pattern
(`available | error` + unsupported shapes; `external_integration_policy` carries mode/disable
semantics): on-demand, cached detection deriving from the order **explicit configured path →
system PATH → managed runtime component**, component = upstream portable binary pinned to a
tag under `runtime_root/wordcraft/<tag>/` with `SHA256SUMS.txt` verification at download
(`src/crates/services/services-core/src/managed_runtime.rs`, resolution helpers already in
`src/crates/services/services-integrations/src/mcp/server/runtime_helpers.rs`). UI entries and
the agent's first step both read this one projection; failure = reason + remediation + Recheck,
never silent fallback. Verification: focused tests in `services-core`/`services-integrations`
per the nearest local AGENTS.md.

## Answer

Landed in `openbitfun-services-core` behind the new `wordcraft` feature (composed with the
existing `process-runtime` owner):

- `src/crates/services/services-core/src/wordcraft.rs` — the factual record + probe:
  - `WordcraftStatus` = `available | unsupported | not-installed | error` (camelCase serde DTO);
  - `WordcraftCapability` carries `cli`/`app` `ResolvedCommand`s (mirrors the resolver's
    explicit-path → PATH → managed order), optional `--version` string, English-only
    `message`/`remediation`, and check timestamp;
  - `WordcraftProbe` = on-demand cached projection (`current()` caches first probe; `recheck()`
    re-derives with a version probe; `invalidate()` forces fresh) — the Recheck backing for UI
    and the agent's first-step answer;
  - ENOEXEC probe failure classifies as `unsupported` (wrong-arch binary), probe failure with a
    CLI-only install stays `available` headless-only (matches the day-one live-if-open rule);
  - `checksum::verify_directory_against_sha256sums` (sha2-backed) verifies an extracted managed
    component against upstream `SHA256SUMS.txt` and refuses unlisted extras — the
    provision-at-download gate ticket 01 requires; actual transfer is ticket 14's design.
- `managed_runtime.rs` — registered the `wordcraft` managed component (`wordcraft`/`wordcraft-cli`
  resolved at `<runtime_root>/wordcraft/current/...`, exe alias normalization, PATH entries).
- Governance mirrors updated for the new feature owner (`scripts/core-boundaries/rules/`
  + checker test fixtures): feature coverage for `sha2`/`libc`/`tokio`/`which`/`win32job`/
  `windows`, the `SERVICES_CORE_TOKIO_FEATURES` contract, the lib.rs source gate regex, and the
  protected-module list.

Verification at the owner (services-core AGENTS.md commands):

```bash
cargo test -p openbitfun-services-core --no-default-features --features wordcraft --lib wordcraft   # 6/6 pass
cargo test -p openbitfun-services-core --no-default-features --features process-runtime --lib managed_runtime  # 3/3 pass
cargo test -p openbitfun-services-core --no-default-features --features wordcraft --lib             # 32/32 pass
cargo check -p openbitfun-services-core --no-default-features --features process-runtime            # clean
pnpm run check:core-boundaries          # pass
pnpm run check:core-boundaries:test     # 148/148 pass
```

Carried over: policy gating (mode/disable via `external_integration_policy`) happens at the
policy owner once the entry points exist (ticket 12); nothing in this record invents an
availability answer.

