# 23: Cross-platform build + CI

**What to build:** Verified build on all three desktop platforms with CI integration.

**Blocked by:** 12 (sidecar config), 13 (model management), 21, 22 (E2E tests)

**Status:** ready-for-agent

- [ ] Local verification:
  - `pnpm run desktop:dev` on Linux (x86_64, aarch64 if available)
  - `pnpm run desktop:dev` on macOS (x86_64, arm64)
  - `pnpm run desktop:dev` on Windows (x86_64)
  - Verify sidecar binaries bundled correctly per platform
  - Verify Basemind sidecar spawns and responds to health check
- [ ] CI pipeline (GitHub Actions):
  - Linux build: `cargo build --release --features basemind,tools-rag,document-read`
  - macOS build: same + notarization
  - Windows build: same + code signing
  - Run unit tests: `cargo test -p basemind-integration`, `cargo test -p alex_desktop basemind`
  - Run E2E tests: `pnpm --filter web-ui test:e2e redacted-document`, `pnpm --filter web-ui test:e2e prompt-pseudonymization`
  - Upload sidecar binaries as artifacts per platform
- [ ] Verification checklist (from spec §16):
  - [ ] Rust (basemind-integration): `cargo test -p basemind-integration`
  - [ ] Services-core: `cargo test -p alex_services_core workspace`
  - [ ] Tool-execution: `cargo test -p alex_tool_execution document`
  - [ ] Assembly/core (tools): `cargo test -p alex_assembly_core rag_search`
  - [ ] Desktop (Tauri commands): `cargo test -p alex_desktop basemind`
  - [ ] Desktop (pseudonymization): `cargo test -p alex_desktop prompt_pseudonym`
  - [ ] Web UI (unit): `pnpm --filter web-ui test BasemindAPI`
  - [ ] Web UI (unit): `pnpm --filter web-ui test localModelDetection`
  - [ ] Web UI (E2E): `pnpm --filter web-ui test:e2e redacted-document`
  - [ ] Web UI (E2E): `pnpm --filter web-ui test:e2e prompt-pseudonymization`
  - [ ] Cross-platform: `pnpm run desktop:dev` (Linux/macOS/Windows)
  - [ ] Remote workspace: `pnpm run test:remote-workspace` (if exists)