# Open Interpreter binary distribution

OpenBitFun packages the Open Interpreter coding agent with the product, so a
user does not have to install it before an ACP session can use it. The agent
speaks ACP natively (`interpreter acp`), so it needs no ACP adapter package.

Nothing binary belongs in Git. `VERSION.json` pins the repository, tag, asset
name, size, and SHA-256 for every supported target.

## What is packaged

A release asset is a `tar.gz` holding the whole package, not a lone executable:

```text
bin/interpreter          the entrypoint
bin/i                    the short alias
bin/codex-code-mode-host
codex-path/rg            ripgrep, used by the agent's file search
codex-resources/         platform helpers (zsh on macOS, bwrap on Linux)
codex-package.json       the upstream package descriptor
```

The helper files are read relative to the executable, so the package has to
ship as one directory. The build step writes `openbitfun-bundled-tool.json`
next to them, which is what the product reads at runtime — the product never
reads the upstream `codex-package.json`, so a change to the upstream layout
cannot break the lookup.

## Running the preparation

```sh
node scripts/prepare-interpreter-resource.mjs
node scripts/prepare-interpreter-resource.mjs --target x86_64-pc-windows-msvc
```

The preparation step requires `curl` (`curl.exe` on Windows) and `tar`. Desktop
builds use the explicit Tauri target, falling back to the Rust host triple.
Downloads go to ignored files under `resources/interpreter/<target>/`. Cached
files are verified before reuse; a failed or corrupted download never becomes
a package input. Only the selected target is added to the Tauri resources, even
if the cache holds other targets.

The asset is large — roughly 220–260 MB compressed per target — so the
download is the slowest part of a desktop build. It is cached between builds.

## Updating the pinned release

Open Interpreter publishes one release per platform under the `rust-v` tag
prefix, with the same version in every asset. Publish the release, verify it,
then update `VERSION.json` with the new tag, version, and the asset
checksums/sizes. Do not use `latest` and do not replace the assets of a
release that has already been consumed; publish a new version.

The digests are the `digest` field GitHub publishes for each release asset.

## Remote workspaces

The packaged copy serves the machine that runs OpenBitFun. A remote SSH
workspace runs the agent on the remote host, so that host needs its own
Open Interpreter; OpenBitFun does not upload a copy there, and says so when
the agent is missing.

## Focused verification

```sh
node --test scripts/prepare-interpreter-resource.test.mjs scripts/desktop-tauri-build.test.mjs
```
