# Open Interpreter as an Alex agentic backend — investigation

Date: 2026-10-19
Status: **investigation complete; additive integration implemented** (see §6)
Target: <https://github.com/openinterpreter/openinterpreter>
Question: can it replace, or be used to replace, the existing Alex agentic backend?

---

## TL;DR

**Do not replace the native backend with Open Interpreter. Do add it as one more ACP
external-agent backend, and seriously consider stealing its "harness emulation" idea.**

Three findings drive this:

1. **Open Interpreter is not the thing it used to be.** The 68.6k-star repo is now a
   **Rust fork of OpenAI's Codex CLI** (`codex-rs/`, `codex-cli/`, and a `NOTICE` file that
   reads "OpenAI Codex, Copyright 2025 OpenAI"). The original Python "let LLMs run code"
   library is effectively dead — it survives as a community fork at
   [`endolith/open-interpreter`](https://github.com/endolith/open-interpreter). Anyone
   evaluating "Open Interpreter" from pre-2026 documentation or blog posts is evaluating a
   product that no longer exists.

2. **It is a product, not an embeddable runtime.** The Rust rewrite ships as a CLI binary
   at **0.0.43**, based on upstream Codex `rust-v0.154.0`, roughly four months old and moving
   fast. There is no library to import. "Using it as the backend" necessarily means spawning
   a subprocess and speaking a protocol to it — which is *exactly* the integration pattern
   Alex already ships for Claude Code, Codex, OpenCode, omp, and dsh.

3. **Alex already has the seam.** `src/crates/interfaces/acp` (13.3k LOC) spawns
   external agent processes over ACP and re-emits the **same `agentic://` event stream** the
   Web UI consumes. The UI contract is the event stream, not the core loop. Adding Open
   Interpreter is a new `BuiltinAcpClientPreset` entry — a few lines plus tests — not an
   architectural change.

---

## 1. What Open Interpreter actually is today

| Attribute | Value |
|---|---|
| Repo | `openinterpreter/openinterpreter`, Apache-2.0 |
| Stars / forks | 68.6k / 5.9k |
| Primary language | Rust (a fork of OpenAI Codex) |
| Version | `0.0.43`, baseline upstream Codex `rust-v0.154.0` |
| Rough age | Repo created 2023-07; the Rust rewrite began ~June 2026 |
| Positioning | "A coding agent optimized for low-cost models" — Kimi K3, GLM 5.3, DeepSeek, Qwen |
| Original Python project | Retired; lives on as community fork `endolith/open-interpreter` |

The README states it plainly: *"Note: this is the new Rust version of Open Interpreter,
based on Codex. Looking for the original Python project? It lives on as a
community-maintained fork."*

Top-level layout confirms the heritage: `codex-rs/`, `codex-cli/`, `bazel/`, `patches/`,
`third_party/`, plus an `AGENTS.md` that is still titled `# Rust/codex-rs` and is written in
Codex's voice. The `NOTICE` file credits OpenAI Codex.

### 1.1 Interfaces it exposes

- **`interpreter acp`** — an ACP agent over stdio. Session create/close/load/list, streaming
  assistant text and reasoning, tool progress, permission requests, model + reasoning
  controls, sandbox modes (`read-only`, `workspace-write`, `danger-full-access`).
- **Codex exec protocol** — a drop-in binary replacement for `codex`; the docs advertise
  `new Codex({ codexPathOverride: "interpreter" })`.
- **`interpreter exec`** — non-interactive, NDJSON events on stdout, `--output-schema`,
  `--resume <SESSION_ID>`, `--verify`, `--sandbox`, `--ask-for-approval`.
- **App server** — JSON-RPC (v1 + v2, per the repo's `AGENTS.md`).
- **MCP server** — expose Open Interpreter as a callable tool to other agents.

### 1.2 The distinctive feature: harness emulation

`/harness` swaps the **model-facing surface** — system prompt, tool schema, message
conversion, response handling — to match the shape a vendor tuned its model on, while the
**native Open Interpreter Rust runtime still executes the tools**. The docs are explicit:

> "The public runtime does not shell out to the real external agent CLI."

Harness IDs: `claude-code`, `claude-code-bare`, `zcode`, `kimi-code`, `kimi-cli`,
`qwen-code`, `deepseek-tui`, `swe-agent`, `minimal` (plus unset = native Responses / Chat).

There is also **automatic harness inference** from the provider/model family (Anthropic →
`claude-code`, Kimi/Moonshot → `kimi-code`, Qwen/DashScope → `qwen-code`, DeepSeek →
`claude-code-bare`), and strict wire-API/harness route compatibility (`responses` | `chat` |
`messages`).

This is the product's actual differentiator and the reason it gets usable work out of cheap
models. It is also the single most transferable idea in the whole codebase (see §4).

### 1.3 Other notable properties

- **Portability-first** (`docs/portability.md`): reads `AGENTS.md`, `.agents/skills/`,
  `~/.agents/skills/` in place; speaks MCP, ACP, and the Codex exec protocol; reserves
  `~/.openinterpreter` for config/credentials/session state only. Explicit product principle
  of "no private island."
- **Native sandboxing** on macOS (Seatbelt), Linux, Windows.
- **Subagents, skills, hooks, permissions, execpolicy, web search, QA/computer-use skill**
  (via `agent-browser` and `trycua`).
- **Companion projects**: `interpreter-workstation` (desktop app), `interpreter-cua`,
  `01` (voice).

### 1.4 Maturity signals — read these honestly

- **0.0.x and four months old.** Inherited upstream Codex churn; releases are frequent.
- A third-party review notes release `0.0.38` **hung the in-session model picker** for signed-in
  users, with "reinstall 0.0.34" as the documented workaround (#1875).
- 10 open issues, 6 PRs, 463 watchers — small maintainer surface relative to the star count.
- The project's own `AGENTS.md` is largely upstream Codex's, including constraints like
  "resist adding code to `codex-core`" and an 800-line change-size guidance — signs of a fork
  still tracking upstream closely rather than diverging.

None of this makes it a bad tool. It makes it a **bad thing to bet your own agent kernel on**.

---

## 2. What the Alex agentic backend actually is

(Condensed from a full survey of `src/crates/`, see the layer map below.)

| Concern | Crate | Size |
|---|---|---|
| Portable runtime facts/ports/facade | `src/crates/execution/agent-runtime` | ~35.7k LOC |
| **Turn loop + tools + sessions + hooks + compaction** | `src/crates/assembly/core` → `src/agentic/**` | **195k LOC inside a 308k LOC crate** |
| Provider adapters | `src/crates/adapters/ai-adapters` | ~31k LOC |
| Tool contracts / execution | `src/crates/execution/tool-*` | ~23k LOC |
| Ports, events, product domains | `src/crates/contracts/*` | ~38k LOC |
| **External agent backends (ACP)** | `src/crates/interfaces/acp` | ~13.3k LOC |
| Ecosystem *source* adapters (not runtimes) | `src/crates/adapters/{opencode,claude-code,codex,dsh,pi}-adapter` | ~25k LOC |

The turn loop proper lives in `assembly/core/src/agentic/execution/`:

- `execution_engine.rs` (6,578 LOC) — outer round loop at line 3397: cancel check →
  token-pressure compression (L1 AI-summary / L2 emergency truncation) → build `RoundContext`
  → `round_executor.execute_round_with_lifecycle(...)` → context-overflow recovery (≤2) →
  append messages → **loop detection** (consecutive-failure + periodic A-B-A-B signatures) →
  steering injection → termination decision → optional finalize round.
- `round_executor.rs` (2,747 LOC) — one model round: stream with retry, then
  `tool_pipeline.execute_tools(...)`, `has_more_rounds = !tool_result_messages.is_empty()`.

Everything around it is what makes it a *product kernel* rather than a loop:

- **48 built-in tools** (`exec_command`, file read/write/edit, glob/grep, `task/` subagents,
  `worktree`, `browser_control`, `computer_use`, `miniapp_*`, `canvas_tools`, MCP, …) behind a
  `Tool` trait with JSON-Schema params, `permission_intents()`, concurrency-safety, and
  deferred two-phase loading (`GetToolSpec` / `CallDeferredTool`).
- **Permission system** — `PermissionMode`, `PermissionRule`, `PermissionEffect`, grants,
  audit, delegation to subagents, and a `PermissionRequestManager` built for **remote**
  answering (broadcast events + snapshots for reconnecting clients).
- **Native hooks** on the **Codex hook contract**, with pre-tool-use hooks able to deny,
  pre-approve, or rewrite input *before* permission planning.
- **Session persistence** (`sessions/<id>/turns/*.json`), fork, revert, restore, manual +
  automatic compaction with immutable message-ID semantics.
- **Event system** — ~44 `AgenticEvent` variants projected to `agentic://*` URI channels,
  fanned out to desktop Tauri, CLI, app-server, remote-connect relay, and mobile, with a
  `session_event_journal` for cursor replay and backfill.
- **Four first-class remote scenarios** (remote workspace, remote control, peer device mode,
  detached dispatch) governed by the **Product Operation Registry** and `PeerHostCapability`.

### 2.1 The precedent that matters

`src/crates/interfaces/acp/src/client/` already implements a **complete external agent
backend**: `AcpClientService` (`client/manager.rs`, 2,971 LOC) drives `initialize` →
`session/create` → `session/prompt` with a streaming `session/update` loop, permission
round-trips, model/config options, context usage, and plan updates — over real JSON-RPC on
stdio.

`client/builtin_clients.rs` registers five presets:

| id | command | install |
|---|---|---|
| `opencode` | `opencode acp` | npm `opencode-ai` |
| `dsh` | `dsh --profile alex-acp` | npm `@deepseek-ai/dsh` |
| `omp` | `omp acp` | **user-managed** (`install_package: None`) |
| `claude-code` | `npx …/claude-agent-acp@latest` | npm, adapter required |
| `codex` | `npx …/codex-acp@latest` | npm, adapter required |

Critically, `src/apps/desktop/src/api/acp_client_api.rs` re-emits **the same frontend
events** as the native loop — `agentic://dialog-turn-started`, `agentic://model-round-started`,
`agentic://text-chunk`, `agentic://tool-event`, `agentic://dialog-turn-*`. **The UI contract
is the event stream, not the core loop.**

Known limitation of the ACP path (from the survey): it **bypasses** `ExecutionEngine` /
`SessionManager`, so compaction, native hooks, permission *policy*, edit-constraint guard, and
loop detection do not apply to external agents. It also refuses interrupted-turn recovery for
externally-routed agents (`coordinator.rs:7199`).

---

## 3. Can Open Interpreter replace the Alex backend?

### 3.1 Full replacement — **No**

| # | Blocking reason |
|---|---|
| 1 | **No embeddable artifact.** The Python library is gone. The Rust version is a CLI + protocols. "Replacement" can only mean "subprocess + protocol," i.e. an ACP integration — which is §3.2, not a replacement. |
| 2 | **Scope mismatch.** Alex's kernel owns sessions, fork/revert/restore, compaction, permission policy + delegation, 48 tools, subagents/swarm, hooks, worktrees, canvas/miniapps, and remote operation. Open Interpreter's ACP surface explicitly gives the *client* the UI and keeps only "model selection, provider transport, instructions, tools, approvals, sandboxing, and session state" for itself. You would lose almost everything above the loop. |
| 3 | **It would regress all four remote scenarios.** No equivalent to the Product Operation Registry, the permission mailbox, the event journal/replay, peer-device capability advertisement, or detached dispatch's durable-job ownership. Per repo rules, "a capability that assumes UI, process, and filesystem share one machine is incomplete." |
| 4 | **Session/state incompatibility.** Open Interpreter persists under `~/.openinterpreter`; Alex's transcript, undo, fork, relay, and restore all read `sessions/<id>/turns/*.json`. An external loop that doesn't write it loses history and remote replay. |
| 5 | **Maturity risk.** Betting a 195k-LOC product kernel — and your users' sessions — on a four-month-old 0.0.x fork of a fast-moving upstream, with a documented recent hang regression, is a poor trade. |
| 6 | **You'd trade your own fork problem for someone else's.** Open Interpreter is a Codex fork still tracking upstream closely (its own `AGENTS.md` says "resist adding code to `codex-core`"). Adopting it wholesale means inheriting *their* upstream-merge burden instead of owning your own loop. |

### 3.2 Additive replacement — **Yes, cheaply (recommended)**

Register Open Interpreter as a sixth ACP external-agent backend. It is the same shape as the
existing `omp` preset: **native ACP, user-managed install, no adapter package.**

```rust
BuiltinAcpClientPreset {
    id: "open-interpreter",
    command: "interpreter",
    args: &["acp"],
    tool_command: "interpreter",
    install_package: None,   // user-managed: curl install script / package manager
    adapter_package: None,   // native ACP over stdio
    adapter_bin: None,
    bundled_profile: None,
},
```

**Why this works with almost no new code:**

- `AcpClientService` already handles session lifecycle, streaming, permissions, model/config
  options, context usage, and plan updates. Open Interpreter's ACP agent advertises all of
  those (see §1.1).
- `acp_client_api.rs` already projects the stream onto `agentic://*`, so the Web UI, mobile,
  and relay paths light up for free.
- Open Interpreter's ACP docs confirm it speaks plain stdio ACP and expects the client to own
  the UI — the exact contract `AcpClientService` was written against.
- `install_package: None` matches the `omp` precedent: Alex detects `interpreter` on
  PATH and launches it; the user runs
  `curl -fsSL https://www.openinterpreter.com/install | sh`. (No npm package exists, so a
  one-click installer is not possible — same situation as omp's bun-based install.)

**What a user actually gains by enabling it:** Open Interpreter's harness emulation —
`kimi-code`, `qwen-code`, `deepseek-tui`, `zcode`, `swe-agent`, `claude-code-bare` — as
selectable agents inside Alex's UI, with Alex's sessions, events, remote surfaces,
and tool cards intact. That is a real capability Alex does not have today: **getting
genuine work out of cheap local/open models** by matching the harness they were tuned on.

**Cost estimate:** one preset struct + a round of the existing `builtin_*` tests + a docs row.
Nothing in `assembly/core`, `agent-runtime`, or `ai-adapters` changes. Verification is the
existing `cargo test -p alex-acp`.

**Caveats to record:**

- The ACP path's existing limitations apply (no compaction, no native hooks, no permission
  *policy*, no loop detection, no interrupted-turn recovery for external routes). This is
  already true for the other five agents; it is not a new regression.
- Open Interpreter is 0.0.x. Pin/document a known-good version and treat the preset as
  user-managed so an upstream regression is the user's to roll back, not Alex's.
- Remote-workspace stance: Open Interpreter's sandbox/approval config lives in its own
  `config.toml`, so if this preset should support remote workspaces it needs a row in the
  **Product Operation Registry** like any other desktop Tauri surface — but note the ACP
  client path already reuses local config for remote workspaces per
  `src/crates/interfaces/acp/AGENTS.md`.

### 3.3 The other direction — **already exists**

Alex already exposes *itself* as an ACP agent (`interfaces/acp/src/server.rs` +
`runtime/**`, `DeliveryProfile::Acp`) to hosts like Zed. Open Interpreter does the same
(`interpreter acp`). **The two products already meet at the protocol boundary**; they don't
need to converge architecturally to interoperate.

---

## 4. The idea worth stealing: harness emulation

This is the highest-value part of the investigation, independent of whether you adopt the tool.

Alex's provider layer has a **closed** wire-format enum:

```rust
// src/crates/adapters/ai-adapters/src/client/format.rs
enum ApiFormat { OpenAIChat, OpenAIResponses, Anthropic, Gemini, GeminiCodeAssist }
```

Adding a provider protocol means editing adapter code (`providers/<x>/{request,message_converter,discovery}.rs`
+ a stream handler + a `resolve_request_url` arm). There is no dynamic registration. The
survey flagged this as a hard-coded assumption.

Open Interpreter's answer is conceptually different and better for the cheap-model problem:
**don't add a wire format per provider — add a model-facing *surface* per model family.**

A harness is: system prompt + tool schema + message conversion + response handling, swapped
per model family, while the *same* runtime executes the tools. `kimi-code` isn't a new API;
it's "send Chat Completions in the shape Kimi K3 was RL-tuned against, but run the resulting
tool calls in our own runtime."

**What this could look like in Alex:**

- The repo already has the *precedent in a different layer*: `claude-code-adapter`,
  `codex-adapter`, `opencode-adapter` import external agent **definitions** (instructions,
  commands, tools, subagents, MCP, hooks) read-only. Harness emulation is the runtime-level
  generalization — import the model-facing *surface*, not the agent.
- Concretely: a `HarnessProfile { system_prompt, tool_schema_shaping, message_conversion,
  response_handling }` selected by model family, applied at the `build_ai_messages_for_send` /
  `RoundContext` boundary in `execution_engine.rs`, orthogonal to `ApiFormat`.
- This keeps the Alex runtime (tools, permissions, hooks, compaction, remote) intact —
  the exact thing Open Interpreter's harness doc says it does ("tool calls still run inside
  Open Interpreter's native Rust runtime").

**Caveat:** this is a real design effort, not a port. Open Interpreter's harness profiles are
tuned against its own tool set and prompts; you'd be re-deriving them for Alex's 48
tools and prompt scaffolding. It is also the part most likely to churn as vendors ship new
model-tuned surfaces. Treat it as a design topic with its own port/provider review, per
`docs/architecture/product-architecture.md`.

---

## 5. Recommendation

1. **Do not replace the native agent kernel.** Open Interpreter is a product, not a runtime,
   and Alex's kernel owns far more than the turn loop — especially everything the four
   remote scenarios depend on.
2. **Do add it as a sixth ACP external-agent backend** (`interpreter acp`, user-managed
   install), mirroring the `omp` preset. Small, safe, reversible, and it gives users
   Open Interpreter's low-cost-model harnesses inside Alex's existing UI and remote
   surfaces.
3. **Pursue harness emulation as a separate design topic** for the native loop. That is the
   durable strategic value here — it addresses a real, identified weakness (closed `ApiFormat`,
   per-provider `message_converter.rs`) that no amount of ACP integration will fix.
4. **Do not** treat Open Interpreter as a dependency to track closely. It is 0.0.x on a fork of
   fast-moving upstream; keep the integration at arm's length behind the ACP protocol, which is
   the one boundary both products already agree on.

### Suggested next steps

- [x] Prototype the `BuiltinAcpClientPreset` entry and run
      `cargo test -p alex-acp --no-default-features --features client`.
- [ ] Manually verify `interpreter acp` streams tool progress + permission requests in the
      Web UI via the existing `acp_client_api.rs` projection.
- [ ] Decide the remote-workspace stance and, if needed, add a Product Operation Registry row.
- [ ] Open a design discussion for `HarnessProfile` in `ai-adapters`, reading
      `docs/architecture/product-architecture.md` and
      `docs/architecture/rust-build-dependency-boundaries.md` first.

---

## 6. Implementation status

The additive integration (§3.2) is **implemented**. The native agent kernel is untouched.

### 6.1 Changes

| File | Change |
|---|---|
| `src/crates/interfaces/acp/src/client/builtin_clients.rs` | New `BuiltinAcpClientPreset` for `interpreter` (native ACP via `interpreter acp`, user-managed install, no adapter, no bundled profile) + `interpreter_is_a_native_acp_preset` test. |
| `src/crates/interfaces/acp/src/client/manager.rs` | `ensure_self_managed_agent_available_remote` — a self-managed agent is checked on the remote host with the same `command -v` question the agent list asks, so a missing binary is reported as a state the user can act on instead of a bare exec failure. Plus `needs_remote_presence_check` and `missing_self_managed_agent_error`, with tests. |
| `src/web-ui/src/infrastructure/config/components/AcpAgentsConfig.tsx` | `interpreter` added to `PRESETS`; added to `SELF_MANAGED_INSTALL_PRESET_IDS` (no npm package → no one-click install) and to `NATIVE_ACP_PRESET_IDS`; `getPresetDescription` case; `SELF_MANAGED_INSTALL_DOC_URLS` + a vendor install-docs link on the "View instructions" action. |
| `src/web-ui/src/infrastructure/config/components/AcpAgentsConfig.test.tsx` | Three new tests (native/self-managed presentation, no downloadable adapter for a native agent, manual install instead of a one-click one); `translate` turned into a spy so interpolation params are assertable. |
| `src/web-ui/src/locales/{en-US,fr-FR,zh-CN,zh-TW}/settings/acp-agents.json` | New `presets.interpreter.description` in all four locales. |
| `src/shared/interactive-capabilities/catalog.json` | Preset list in the ACP capability item now names Open Interpreter (zh + en, plus the embedded `reasonZh`/`reasonEn` strings). |
| generated artifacts | `capabilities:generate` output: `docs/interactive-capabilities/**`, `src/web-ui/src/app/global-search/generated/interactive-capabilities.json`, `src/web-ui/src/infrastructure/api/generated/productControl.ts`, `src/crates/contracts/product-domains/src/generated/product-control-catalog.json`. |

### 6.2 Defects found and fixed during the production-readiness pass

1. **`NATIVE_ACP_PRESET_IDS` drift** (`AcpAgentsConfig.tsx:102`) — the Web UI mirror of the Rust
   preset table listed `opencode`/`dsh`/`omp` as native-ACP but not `interpreter`. Because
   `requiresAdapter` is computed as `Boolean(probe?.adapter || !NATIVE_ACP_PRESET_IDS.has(id))`,
   `interpreter` was permanently marked as needing an adapter. Benign today only because the
   Rust probe omits the `adapter` key entirely, making `adapterInstalled` `undefined` rather
   than `false` — the moment the probe starts emitting `adapter: { installed: false }`, the
   agent would show "ACP missing" and offer a "Configure ACP" action that fails with
   "ACP client 'interpreter' does not use a downloadable adapter". Fixed by adding
   `'interpreter'` to the set, and covered by a test.
2. **No way to learn how to install a user-managed agent.** The "View instructions" action
   showed only the command name and a PATH reminder — no link, no install command, for
   `interpreter` or for the pre-existing `omp`. Now the action opens the vendor's own install
   documentation (the vendor owns those pages, so they stay correct across platforms and
   release channels) while the toast keeps the Alex-side guidance.

### 6.3 Remote-workspace stance — decided

**No Product Operation Registry change is required, and none was made.** The registry
(`src/crates/contracts/product-domains/src/remote_surface/table.rs`) is keyed by *Tauri
command*, not by agent id; all 16 ACP client commands already have rows. `interpreter` is a
value of the `client_id` request field, not an operation, so adding an agent cannot affect
the registry, the `Unaudited` backlog ratchet, or the desktop closure test.

The remote path is agent-agnostic and works for `interpreter` as-is: the remote probe
(`manager.rs:432`) enumerates `builtin_client_ids()`, so `interpreter` is probed on every
SSH host with the same `command -v` question the local path asks, and
`resolve_config_for_client` falls back to `interpreter acp` when no config is saved for it.

The one gap — a remote launch of a missing self-managed agent surfacing a bare exec
failure — is now closed by `ensure_self_managed_agent_available_remote`, modelled on the
existing `ensure_bundled_profile_remote` precedent for `dsh`.

Deliberately **not** changed:

- `src/apps/cli/src/acp_cli.rs` — its `ExternalAcpClient` enum is an existing curated subset
  that already omits `dsh` and `omp`; adding Open Interpreter there would break that
  precedent, and it is CLI-surface ownership independent of this integration.
- `ecosystemCompatibilityModel.ts` `EcosystemProductId` — `interpreter` has no
  external-source adapter, and `omp` is excluded from that catalog by an explicit test.
- The native agent kernel, `agent-runtime`, `ai-adapters`, permissions, hooks, and
  persistence — untouched, per §3.1.

### 6.4 Verification

| Gate | Result |
|---|---|
| `cargo test -p alex-acp --no-default-features --features client,alex-core/git --lib` | 83 passed, 0 failed |
| `cargo check -p alex-acp` (default features, client+server) | clean |
| `rustfmt --edition 2021 --check` on both changed Rust files | clean |
| `pnpm run i18n:audit` | passed, 0 warnings |
| `pnpm run capabilities:check` | verified in sync |
| `pnpm run type-check:web` | clean |
| `pnpm run check:web` | exit 0 |
| `pnpm --dir src/web-ui run test:run …/AcpAgentsConfig.test.tsx` | 31 passed, 0 failed |
| `pnpm run check:repo-hygiene` | passed |

Note: the client closure requires Core's `git` feature
(`--features client,alex-core/git`); without it `alex-core` fails to resolve a
git-gated worktree import. That is a pre-existing feature-invocation requirement documented
in `src/crates/interfaces/acp/AGENTS.md`, not a regression from this change.

### 6.5 Still open

- **Runtime smoke test** against a real `interpreter` binary — needs the CLI installed and a
  provider key. The agent must also be configured by the user, because Alex ships no
  installer for it.
- **User-facing documentation** — there is no canonical user-facing page listing the
  supported ACP external agents today; the only list is the generated capability doc, which
  now names Open Interpreter. A new `docs/features/acp-external-agents.md` would be a
  docs-coverage addition, not a correctness fix.
- **The `HarnessProfile` design** (§4), which remains the higher-value but larger piece of
  work.

---

## 7. Shipping Open Interpreter with the product

The product now packages the Open Interpreter agent, so a user does not have to install it
before an ACP session can use it.

### 7.1 What is packaged, and how big it is

A release asset is a `tar.gz` holding the whole package, not a lone executable:

```text
bin/interpreter          the entrypoint
bin/i                    the short alias
bin/codex-code-mode-host
codex-path/rg            ripgrep, used by the agent's file search
codex-resources/         platform helpers (zsh on macOS, bwrap on Linux)
codex-package.json       the upstream package descriptor
```

The helper files are read relative to the executable, so the package ships as one
directory.

**The cost is real and should be stated plainly:** the asset is **220–260 MB compressed per
target** (Open Interpreter 0.0.56), against roughly 8–10 MB for the Flashgrep binary the
product already packages. The uncompressed package is larger still. Every desktop release
artifact therefore grows by a few hundred megabytes, and the release build gains a
220–260 MB download per platform (cached between builds — the measured cold run was 36 s
on a developer machine).

### 7.2 The shape

The build step follows the Flashgrep precedent, which is the closest existing pattern: a
prebuilt third-party binary, downloaded and verified at build time, injected into the
Tauri bundle as a resource, and resolved from the product's resource directory at runtime.

| Layer | File | Change |
|---|---|---|
| Services | `src/crates/services/services-core/src/bundled_tools.rs` | New reusable service: discovery of tool packages that ship inside the product bundle. |
| Assembly | `src/crates/assembly/core/src/service/bundled_tools/mod.rs` | Re-export, so product surfaces use one stable path. |
| Interfaces | `src/crates/interfaces/acp/src/client/bundled_tool.rs` | Resolves a client's command to the packaged binary, or the user's own when they changed it. |
| Interfaces | `src/crates/interfaces/acp/src/client/builtin_clients.rs` | `bundled_tool` on the preset. |
| Build | `scripts/prepare-interpreter-resource.mjs` | Download, verify, extract, and write the package manifest. |
| Build | `scripts/desktop-tauri-build.mjs` | Inject the package as a Tauri resource; sign every executable in it on macOS. |
| Packaging | `Alex-Installer/scripts/build-installer.cjs` | Carry the package into the Windows payload and validate its entrypoint. |

### 7.3 Decisions worth recording

1. **The product reads its own manifest, not the upstream one.** The build step writes
   `alex-bundled-tool.json` next to the packaged files. A change to the upstream
   `codex-package.json` layout therefore cannot break the runtime lookup.
2. **A host tag gates the package.** The manifest records the host the package was built
   for (`macos-aarch64`, `linux-x86_64`, …) and the runtime only launches a package whose
   tag matches the machine it is running on. The tag follows the **target**, not the build
   host, so a cross-compiled package is labelled for the machine it will run on.
3. **The packaged copy is a default, not an override.** A client whose command the user
   changed keeps the user's command. The packaged binary is what Alex ships and
   tested; it does not shadow a deliberate choice.
4. **Remote workspaces are not provisioned.** The packaged copy serves the machine running
   Alex. A remote SSH workspace runs the agent on the remote host, which needs its
   own installation; the remote launch error says exactly that. Uploading a ~500 MB
   package per remote host is a different feature, not a packaging detail.
5. **The CLI distribution does not carry it.** The CLI ships no packaged binaries today
   (not even Flashgrep), so the CLI falls back to the user's own installation.

### 7.4 Licensing

Open Interpreter is Apache-2.0 and is a fork of OpenAI Codex. Apache-2.0 requires the
license and the upstream NOTICE to travel with the redistributed code, so the build step
copies `resources/interpreter/LICENSE` and `resources/interpreter/NOTICE` into the package
itself, and `THIRD_PARTY_NOTICES.md` records the redistribution.

### 7.5 Pinned release

`resources/interpreter/VERSION.json` pins the repository, the `rust-v0.0.56` tag, and the
size and SHA-256 of every target asset, taken from the `digest` field GitHub publishes for
each release asset. A download that does not match never becomes a package input.

### 7.6 Pre-existing breakages found on the way

Two failures existed on `HEAD` before this work and had to be fixed to compile and test
anything:

1. `alex-core` did not compile: `commit 2f9c4a3eb` ("make French the product default
   locale") added a non-`const` `Default for LocaleId` while `app_language.rs` uses
   `LocaleId::default()` in a `const`. The repository rule "Keep `cargo check -p
   alex-core --no-default-features` viable" was being violated. The upstream author
   then fixed it properly in `9a31b9004` by exposing the contract default as a
   `DEFAULT_LOCALE` const, so no local change was needed.
2. `scripts/desktop-tauri-build.test.mjs` fails on `main`: the locale contract lists
   `fr-FR`, but the product-customization fixture `products/fixtures/acme/locales/` has no
   `fr-FR.json`, and the resolver requires a file for every supported locale. This one is
   fixed in the pull request, because the branch would otherwise carry a red test in a file
   the change touches.

---

## 8. Review findings and their resolutions

A Standards and Spec review ran against the diff before the pull request was opened.

| # | Axis | Severity | Finding | Resolution |
|---|---|---|---|---|
| 1 | Spec | Important | The packaged macOS `zsh` helper (`codex-resources/zsh/bin/zsh`) was never signed, because the executable list was hardcoded. An unsigned nested binary fails notarization. | **Fixed.** The executable list is now derived from the upstream package descriptor — the entrypoint, its directory, and the `pathDir`/`resourcesDir` the upstream names. Verified against the real package: the list now includes `codex-resources/zsh/bin/zsh`, and the data files do not. |
| 2 | Spec | Important | The remote "missing agent" error told every self-managed agent that Alex packages a copy — false for `omp`, which is not packaged. | **Fixed.** The message now mentions the packaged copy only for an agent that has one, and says plainly what to do otherwise. Covered by a test for both cases. |
| 3 | Spec | Important | Every desktop build downloaded 220–260 MB, including debug builds. | **Fixed.** A debug build does not pull the package, and `OPENBITFUN_SKIP_INTERPRETER_PACKAGE=1` opts a release build out. Release and installer builds still carry the package. |
| 4 | Standards | Important | The executable set was hardcoded while the entrypoint was dynamic, so a rename upstream would leave the entrypoint unsigned and unexecutable. | **Fixed** with finding 1. |
| 5 | Standards | Nit | `copyDirectory` marked every copied file executable, including the license and the package descriptor. | **Fixed.** The mode now travels with the file. |
| 6 | Standards | Nit | `readPinnedManifest` ignored a caller-supplied manifest, so a test silently asserted against the repository's real pinned release. | **Fixed.** A caller-supplied manifest is honoured, and a test covers it. |
| 7 | Standards | Nit | Three statements were mangled into one line by an earlier edit. | **Fixed.** |
| 8 | Standards | Judgement call | The agent facts are duplicated between the Rust preset table and the Web UI preset table, so adding an agent touches both. | **Addressed at the source.** The two lists are now checked against each other by a surface test, so the next agent cannot be added to one and missed in the other. The lists were not merged: that is a larger refactor of a pre-existing structure, and this change did not introduce it. |
| 9 | Standards | Beyond the nit cap | `products/fixtures/acme/locales/fr-FR.json` is an unrelated fix. | **Kept, with a reason.** It is a three-line file, it fixes a test that is already red on `main`, and without it the branch carries a failing test in a file this change touches. It is called out separately in the pull request. |

The drift check was first written as a Rust test in the ACP crate. The repository boundary
check correctly rejected it — Rust source must not reference the Web UI source tree — and
the check was moved to the Web UI surface test, which is where the rule says a
cross-surface contract belongs. The check was verified to fail when an agent is added to
only one of the two lists.

### 8.1 CodeRabbit findings

CodeRabbit reviewed the branch and raised five findings. All five were fixed, and each fix
has a test.

| # | Severity | Finding | Resolution |
|---|---|---|---|
| 1 | Minor | The installer joined the manifest's `entrypoint` onto the package root without checking containment, so an entrypoint such as `../resources/worker_host.js` would resolve outside the package. | **Fixed.** The resolved path is checked against the package root before it is accepted, and a tampered entrypoint is rejected. |
| 2 | Minor | The macOS signing step signed the manifest's executable paths without checking containment, so a path could reach a file outside the package. | **Fixed.** A path that climbs out of the package is rejected before anything is signed. |
| 3 | Minor | An interrupted preparation left its staging directory behind. The package is a few hundred megabytes, so every interrupted run kept another copy on the build machine. | **Fixed.** Stale staging directories for the target are swept before the download — before it, so a failed download still frees them — and a staging directory for a different target is left alone. |
| 4 | Minor | The development-tree lookup used `env!("CARGO_MANIFEST_DIR")`, which bakes the build machine's absolute path into every release binary. | **Fixed.** The development tree is discovered at runtime by walking up from the running executable, so a shipped binary carries no build path. |
| 5 | Minor | The runtime joined the manifest's `entrypoint` onto the package root without checking containment. | **Fixed.** An entrypoint containing a parent or root component is refused. |

Findings 1, 2 and 5 are the same class: a manifest is data, and data read from disk has to
be validated before it is used to build a path or to choose a file to sign. The fix is the
same check in each place, because each place builds a path for a different purpose — the
installer validates a payload, the build signs a package, and the runtime launches an
agent.





---

## Sources

- <https://github.com/openinterpreter/openinterpreter> (README, file tree, `NOTICE`, `AGENTS.md`, `RELEASE_NOTES.md`)
- <https://www.openinterpreter.com/docs/terminal/harness>
- <https://www.openinterpreter.com/docs/terminal/acp>
- <https://www.openinterpreter.com/docs/terminal/exec>
- <https://raw.githubusercontent.com/openinterpreter/openinterpreter/main/docs/portability.md>
- <https://agents-oss.com/projects/openinterpreter> (third-party maturity notes, #1875)
- <https://github.com/endolith/open-interpreter> (the retired Python project's community fork)
- Alex survey: `src/crates/assembly/core/src/agentic/`, `src/crates/interfaces/acp/`,
  `src/crates/adapters/ai-adapters/`, `src/crates/execution/agent-runtime/`
