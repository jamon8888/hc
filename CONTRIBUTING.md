# Contributing

[中文版](./CONTRIBUTING_CN.md)

Thanks for your interest in OpenBitFun! OpenBitFun is a multi-platform AI programming environment powered by Rust and TypeScript, with shared core logic across Desktop/CLI/Server. This guide explains how to contribute effectively.

## Code of Conduct

Be respectful, kind, and constructive. We welcome contributors of all backgrounds and experience levels.

## Quick Start

### Prerequisites

- Node.js 22.12+ (LTS recommended)
- pnpm 10.15.0 via Corepack
- Rust toolchain (install via [rustup](https://rustup.rs/))
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for desktop development

OpenBitFun standardizes local JavaScript builds and CI on Node.js 22.12+. The GitHub
Actions upgrades in this repository use Node.js 24-compatible action runtimes,
but project scripts should run on Node.js 22.12+ unless a narrower local guide
says otherwise. After switching from an older Node.js version, rerun
`pnpm install`.

#### Build Prerequisites Check

When `cargo check --workspace`, `cargo check -p openbitfun-desktop`, or pnpm build
commands fail with confusing errors (e.g., "resource path doesn't exist" or
sherpa-onnx download failures), run the preflight check to identify missing
prerequisites and get actionable fix commands:

```bash
pnpm run check:build-prereqs           # check only
pnpm run check:build-prereqs -- --fix  # attempt to fix missing prerequisites
```

The check detects:

- Missing `node_modules` (fix: `pnpm install`)
- Missing `src/mobile-web/dist` (fix: `pnpm run prepare:mobile-web` — the
  openbitfun-desktop Tauri build script references this directory as a resource,
  so `cargo check -p openbitfun-desktop` and `cargo check --workspace` fail
  without it)
- Missing sherpa-onnx prebuilt libs (the sherpa-onnx-sys build script
  downloads from GitHub at build time; if the download fails on poor
  connectivity, set `SHERPA_ONNX_LIB_DIR` to the prebuilt lib directory
  under `target/sherpa-onnx-prebuilt/` to use the local copy)

### Install dependencies

```bash
pnpm install
```

The pnpm workspace uses the root `pnpm-lock.yaml` as its dependency lockfile;
do not commit `package-lock.json` files for workspace packages. The standalone
`packages/dsh-acp` npm package and `src/apps/extension-host` Bun host retain their
own lockfiles and preparation commands. Keep generated Tauri `gen/` directories
and local implementation-process notes out of Git.

### Common commands

```bash
# Desktop (recommended for daily development)
pnpm run desktop:dev                # full hot-reload: Vite HMR + Rust auto-rebuild & restart
pnpm run prepare:dsh-profile        # optional: compile the DeepSeek Harness bridge for local sessions

# Desktop (lightweight preview, no Rust auto-rebuild)
pnpm run desktop:preview:debug      # reuse pre-built binary + Vite HMR; Rust changes require manual restart

# Desktop (production build)
pnpm run desktop:build

# E2E
pnpm run e2e:test
```

> **`desktop:dev` vs `desktop:preview:debug`**: `desktop:dev` runs `tauri dev`, which provides **full hot-reload** — frontend changes apply instantly via Vite HMR, and Rust/backend changes trigger an incremental rebuild followed by an automatic app restart. This is the recommended workflow for active development. `desktop:preview:debug` launches a pre-built debug binary alongside a Vite dev server; frontend edits still get HMR, but **Rust-side changes are not auto-rebuilt** — you must stop and re-run the command (or use `--force-rebuild`). Use `desktop:preview:debug` when you only need to iterate on frontend code or want a faster cold-start without waiting for `tauri dev` initialization.

> For the full script list, see [`package.json`](package.json). For agent-specific commands, verification, and architecture rules, see [`AGENTS.md`](AGENTS.md).

### Desktop debugging tools

Desktop dev builds enable the `devtools` Cargo feature. Use `F12` for native
webview DevTools. `Cmd/Ctrl + Shift + I` toggles the OpenBitFun element inspector,
and `Cmd/Ctrl + Shift + J` also opens native DevTools. These tools are disabled
in end-user `release` builds.

## Code Standards and Architecture Constraints

Use [`AGENTS.md`](AGENTS.md) as the canonical source for architecture-sensitive
rules, module boundaries, and the verification matrix. In contributor-facing
terms:

- Logs are English-only and should stay useful, not noisy.
- User-visible copy should use the project i18n flow; do not share Web UI
  locale catalogs with smaller surfaces.
- Shared core must stay platform-agnostic. Desktop/Tauri details belong in app
  adapters and flow through typed capability interfaces; use the production
  transport adapter when event delivery is needed.
- Tauri commands use `snake_case` command names and structured `request`
  payloads.
- Product architecture, feature-boundary, dependency-boundary, and build-speed
  work must follow `docs/architecture/product-architecture.md`.
- Feature-specific rules belong in the nearest module `AGENTS.md`.

## Key Contribution Focus Areas

1. Contribute good ideas/creativity (features, interactions, visuals, etc.) by opening issues
   > Product managers and UI designers are welcome to submit ideas quickly via PI. We will help refine them for development.
2. Improve the Agent system and overall quality
3. Improve system stability and strengthen foundational capabilities
4. Expand the ecosystem (Skills, MCP, or better support for domain-specific development scenarios)

## Contribution Workflow and PR Expectations

### What to Contribute (Beyond Features and Fixes)

We welcome contributions beyond standard feature or bug-fix PRs. Examples include:

| Contribution area | Location / files | Example |
| --- | --- | --- |
| Prompts | `src/crates/assembly/agent-content/prompts/agents/` | Add or refine built-in prompts; keep selection and runtime policy in their existing owners |
| Tools | `src/crates/assembly/core/src/agentic/tools/implementations/`, `src/crates/assembly/core/src/agentic/tools/registry.rs` | Add tool implementations and register them in the tool registry |
| Subagents | `src/crates/assembly/core/src/agentic/agents/definitions/`, `src/crates/assembly/core/src/agentic/agents/registry/` | Add subagent definitions and register them with the owning registry |
| Mode contributions | `src/crates/assembly/core/src/agentic/agents/definitions/`, `src/crates/assembly/agent-content/prompts/agents/`, `src/web-ui/src/locales/` | Keep mode policy, built-in prompts, and owning UI copy in sync |
| Playbook and scenario guides | `src/shared/interactive-capabilities/catalog.json`, owning app README | Maintain the source catalog and run `pnpm run capabilities:generate`; `website/` consumes the generated catalog |

### Before you start

- Open an issue to describe the problem or proposal, especially for larger changes, to avoid duplication and design conflicts
- For new features or UI changes, discuss the design direction early to ensure it fits the product experience
- Use the issue and PR templates as a guide. Keep the PR focused and explain any skipped verification when it matters.
- For larger or riskier changes, settle the review depth early: the risk tiers and approval counts live in [Code review](#code-review).

### PR title and description

We recommend using Conventional Commits for clearer history and better automation:

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `chore:` maintenance/deps
- `refactor:` refactor without behavior change
- `test:` tests

UI changes should include before/after screenshots or a short recording for fast review.

If your work is AI-assisted, please note it in the PR and indicate testing level (untested/lightly tested/fully tested) to help reviewers assess risk.

Do not commit transient AI prompts, local absolute paths, generated scratch files, pairing secrets, tokens, certificates, or unrelated artifacts. Keep the PR focused on the intended product or maintenance change.

Git objects larger than 5 MiB are rejected by the Repository Object Sizes check,
including files added in an intermediate commit and deleted before the PR's final
commit. Keep build artifacts outside Git. Existing bundled Chinese fonts have
exact-object exceptions in `scripts/git-object-size-policy.json`; changes to those
exceptions require review. Check a proposed history with
`node scripts/check-git-object-sizes.mjs --base origin/main --head HEAD`, or omit
`--base` to check all reachable history. Verify the checker with
`node --test scripts/check-git-object-sizes.test.mjs`.

### Branch management

**The `main` branch is the default collaboration branch and accepts feature PRs.** Since this repo encourages product managers and developers to use AI-generated code for rapid validation or idea submission, **please open all PRs targeting the `main` branch**.

### Scope

Keep PRs small and focused. Avoid bundling unrelated changes.

## Testing and Verification

Run the smallest checks that match the changed files and behavior. CI covers
full builds and broad test suites; local prechecks should stay focused unless
the change affects build, packaging, release behavior, or a path CI cannot
protect.

Common local checks:

| Change type | Typical verification |
| --- | --- |
| Repository metadata or GitHub config | `pnpm run check:repo-hygiene && pnpm run check:github-config && git diff --check` |
| Frontend runtime or UI | `pnpm run check:web`, plus the nearest focused test when behavior changed |
| Mobile web | `pnpm --dir src/mobile-web run type-check` |
| Rust shared runtime or services | Follow the nearest module `AGENTS.md`: one package/test target and the minimum required features |
| Desktop/Tauri integration | `cargo check -p openbitfun-desktop` |
| i18n resources or contract | use the matching i18n row in `AGENTS.md` |

For UI changes, include screenshots or a short recording when helpful. If you
cannot run a relevant check, explain why in the PR and provide a lower-risk
manual verification path.

`pnpm run check:web` combines the Web UI type-check with the Appearance
contract, theme color, and theme visual governance gates that CI applies to
frontend changes.

## Code review

Review runs in layers, not as one gate at the end. Most of the reading is
automated, and human attention goes where the risk is.
[`REVIEW.md`](REVIEW.md) is the reviewer-facing contract: what failure looks
like in this repo, how a finding is written up, and the caps that keep review
converging. Read it before you review, and before you ask someone else to.

### Before you open the PR

- Give the agent checks it can run, and paste their output into the PR's
  Verification section. "Done" is not evidence; the command and its output are.
- Run an agent review against your own diff first, with fresh context: the
  `code-review` skill (Standards against `CONTRIBUTING.md`, `REVIEW.md` and the
  nearest `AGENTS.md`; Spec against the issue under `.scratch/`), plus the
  focused checks from the table above. Fix what it finds before spending a
  person's attention on it.
- Keep the change small. A change that is easy to revert is easy to approve.

### The layers

| Layer | What runs | Before a person looks |
| --- | --- | --- |
| 1. Self-verification | Exact commands and output in the PR's Verification section | Yes |
| 2. Agent review | Fresh-context review of the diff; every finding fixed or resolved with a reason | Yes |
| 3. Automated checks | CI: builds, tests, repository hygiene, GitHub config, core boundaries, i18n and theme audits | Yes |
| 4. Human approval | Scaled by the risk tier below | Yes, for Medium and High |
| 5. Merge and rollback | Small PRs, one-step revert, feature flags where the change is not trivially reversible | After |

### Open Code Review

Layer 2 can run on [Open Code Review](https://github.com/alibaba/open-code-review)
(`ocr`), a deterministic-pipeline plus LLM-agent reviewer that posts line-level
findings. It is optional: the `code-review` skill and a fresh-context subagent
are still valid ways to fill layer 2.

```bash
npm install -g @alibaba-group/open-code-review
ocr config provider      # or non-interactive: ocr config set provider <name>
ocr config model
ocr llm test             # must pass before a review is worth running
ocr review --from main --to <branch>
ocr review --preview     # list the files it would read, no model call
```

- Credentials live in `~/.opencodereview/config.json` (mode `600`), outside the
  repository. Never write an endpoint URL or API key into a committed file —
  `pnpm run check:repo-hygiene` rejects them, and a key in Git needs rotation.
- In OpenCode, `.opencode/plugins/` registers `ocr_review`, `ocr_health`,
  `/ocr-review` and `/ocr-health`. Run `npm install` inside `.opencode/`
  first; its `node_modules` is gitignored.
- `ocr` findings obey the same caps and severity tags as any other review:
  every finding is answered, nits are capped, and repeats get fixed at the
  source rather than reposted.

### Risk tiers

The change sets the tier, not the author. Declare it in the PR template.

| Risk | Examples | Human approvals |
| --- | --- | --- |
| Low | Docs, copy, internal-only helpers, additive change behind a flag, generated-output refresh | None, once layers 1-3 are green |
| Medium | Ordinary product changes; caching, adapters, sessions, settings, remote and peer paths | One |
| High | Auth, credentials, release and packaging, CI workflows, persisted data shapes, upgrade compatibility, security boundaries | Two |

- An agent's approval never counts as a person's.
- Whoever directed an agent-generated change counts as one of the required
  reviewers, exactly as if they had written it.
- An approval covers one commit. New commits — especially an agent's push or a
  hand-resolved merge — need a fresh approval.
- Branch protection and `CODEOWNERS` are how tiers 3 and 4 are enforced
  server-side. Until they are configured, tiers are a review norm the author
  and reviewers hold to.

### Convergence

An agent author and an agent reviewer can keep each other busy indefinitely.
These rules are what stop that:

- **Every finding is answered** — fixed, or resolved with a short reason.
  Silence never closes a finding.
- **Round cap: three.** After three rounds of review and fix, stop and
  reconsider the approach instead of starting a fourth pass.
- **Re-reviews read the new diff only** after the first pass.
- **Reviewers report the class of problem**, so one fix closes every instance,
  and never re-post a finding already posted.

### Fix repeated comments at the source

When a reviewer flags the same thing in two PRs, the fix is not a third
comment. Move it into the thing that runs before the next review:

- a convention → [`REVIEW.md`](REVIEW.md) or the nearest `AGENTS.md`
- something a machine can catch → a lint rule, an audit, or a `scripts/check-*.mjs`
- a stale instruction → the doc line it contradicts
- a bug that shipped → a failure mode in `REVIEW.md`, so the class is caught next time

A revert or an escaped regression is review evidence, not just an incident:
record the failure mode so the next review catches it.

## Security and Compliance

- Do not commit secrets, tokens, certificates, or any sensitive data
- When adding dependencies, ensure license compatibility and explain the purpose

## Thanks

Every contribution matters. Issues, PRs, and suggestions are all welcome!
