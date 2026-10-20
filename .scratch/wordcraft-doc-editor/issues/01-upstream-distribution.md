# upstream-distribution

`Type: research`
`Status: resolved`

## Question

How does OpenBitFun obtain, pin, and ship WordCraft? Compare: (a) vendoring the key upstream
crates (`wordcraft-doc/docx/layout/engine/mcp`) into the workspace, (b) shipping prebuilt
`wordcraft-cli`/`wordcraft` release binaries from upstream GitHub releases, (c) a runtime-fetched
binary (the npx-at-runtime model, like univer's decision). For each: per-platform artifact
availability (macOS x64/arm64, Windows, Linux), version pinning and upgrade-compat story,
what attribution MIT OR Apache-2.0 requires (upstream has an asset-attribution check), binary
size/build-time cost, and what the repo's Cargo dependency-boundary rules
(`docs/architecture/rust-build-dependency-boundaries.md`) imply for (a).

## Answer

**PREBUILT + PINNED, resolved through `ManagedRuntimeResolver`: explicit configured path →
system PATH → managed runtime component** (downloaded at enable/provision time, pinned to an
exact upstream tag, verified against the release's `SHA256SUMS.txt`).

Verified upstream facts (2026-10-16, release `v0.7.0` published 2026-10-14 — fast-moving):

- Not published to crates.io, so "(a) vendoring crates into the workspace" would mean a checked-in
  git- or path-copy of the whole L0–L6 tree. Rejected for day one: it imports upstream's
  dependency tree (egui, wgpu/vello, Krilla/`craft-pdf`, fonts) into our build, and our workspace
  rules (feature policy per crate, no `tokio/full`, build-dependency boundaries, Docker-context
  manifests) would need a per-crate audit for zero product benefit — the engine+MCP binary is
  already the vendor-friendly seam upstream designed (L6 UI is explicitly swappable, commands are
  the stable surface). Keep vendoring as a recorded fallback if a deep in-process API need ever
  appears.
- Per-platform artifacts exist as **portable archives** (no installer required): 32-bit +
  x64/arm64 portable zips/tar.gz for Windows; Linux tar.gz/deb/rpm/AppImage + FreeBSD; macOS
  dmg (app), `wordcraft-cli-0.7.0-macos-universal.zip` (CLI). Massive (~82–100 MB each), so the
  installer must not bundle them per-platform by default; the managed component downloads at
  enable/provision time with checksum verification, exactly the univer ticket-10 pattern
  (`runtime_root/<component>/current`).
- Runtime fetch is unproblematic (contrast with the univer licensing verdict): MIT OR Apache-2.0,
  `LICENSE-MIT` + `LICENSE-APACHE` shipped upstream, no credential. Attribution: redistribute the
  two license texts, credit upstream in our notice/credits list, and — because the portable
  binary ships unmodified — upstream's own bundled-asset attribution travels inside it; we add a
  pointer rather than re-auditing it. Record "unmodified upstream binary" as the rule so rebrand
  is never attempted.
- Pinning/upgrade-compat: pin the managed component to a fixed tag per product release; the bump
  is a deliberate product step, never auto-update. MCP tool names and the control protocol are a
  de-facto contract but upstream makes no compat promise this early — on each pin bump, ticket 07's
  probe (`list_commands` count, `inspect_document`, `engine.execute/commands`) is the smoke test.
  On a failed/missing download the capability record answers verbatim ("unavailable-loud") instead
  of any silent fallback — matches the univer ticket-03 gating decision.
- `wordcraft-web-0.7.0.zip` (14 MB) is its own artifact — ticket 03 evaluates whether it can be
  the embedded editor surface. Note for tickets 05/06: binaries are per-host executables; a
  *remote workspace* scenario would need the binary on the workspace host (same model as the
  univer daemon, differently licensed).

