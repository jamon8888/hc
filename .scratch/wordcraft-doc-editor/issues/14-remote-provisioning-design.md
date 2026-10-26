# remote-provisioning-design

`Type: grilling`
`Status: resolved`

Blocked by: 09, 10

## Question

Ticket 06 expanded scope: the headless server must run **on the remote workspace host**. Design
the provisioning path for the managed WordCraft component onto SSH/jump-host/container targets:
how the pinned artifact (with `SHA256SUMS.txt`) transfers over the workspace transport, where it
installs on the host, how resolution/discovery works there (explicit path → PATH → managed),
POSIX-path discipline end to end (no host `std::path` semantics on client sides), binary refused
or arch/OS mismatch → loud unsupported state surfaced at the driving surface. Deliverables: a
sharpened design decision (per docs/agents ADR-ish answer here) + acceptance states per target
type. Upstream remote-forwarding precedent (`ssh -L` + `WORDCRAFT_CONTROL_KEY`) stays the
documented live-window path, not the headless editing path.

## Answer

**PROVISION-PER-HOST, controller-pushed, flashgrep mechanics + dispatch negotiation: the
pinned target-platform artifact is fetched and verified on the controller, transferred over
the existing workspace transport with the `ensure_remote_flashgrep_binary` two-phase pattern,
installed under a host-managed POSIX runtime root, and gated by a remote capability record in
the same `WordcraftStatus` vocabulary — surfaced loud at the driving surface, never a local
editing fallback.**

### 1. Transfer (reuse the flashgrep pattern, not a new protocol)

Sequence per workspace host (`remote_ssh::wordcraft_provision`, behind `remote-ssh`):

1. **Probe first**: remote OS/arch via the existing `REMOTE_OS_PROBES` /
   `REMOTE_ARCHITECTURE_PROBES` (`uname`-based, normalizes to `x86_64`/`aarch64`), plus
   `command -v tar` (extraction prerequisite). Unknown `(os, arch)` pair or missing `tar` →
   capability record `unsupported` **before** any byte moves.
2. **Fetch controller-side**: download the pinned (`PINNED_WORDCRAFT_VERSION`) *target-platform*
   CLI asset (per-platform mapping from ticket 01's asset table — verify exact asset names at
   each pin bump) and verify it against the release `SHA256SUMS.txt` with the existing
   sha256 verifier from ticket 09. Cache locally under the managed runtime downloads dir keyed
   `wordcraft/<version>/<os>-<arch>` so repeat provisions across workspaces on the same host
   family skip the network. Controller offline or asset fetch failure → record `error` with
   remediation; **never** proceed unverified.
3. **Push**: `provider.write_file` (routes itself: SFTP / `container_write_file` / WSL — the
   transport abstraction already handles target variants) to
   `<install_dir>/.upload-<sha256>.tmp`, then shell-escaped `mv -f`, then remote sha256
   re-check (`sha256sum` with `shasum -a 256` fallback — the exact flashgrep ladder), then
   extract with remote `tar`, then `chmod 755`. Idempotent: a matching remote sha skips
   upload entirely (flashgrep behavior).
4. **Verify**: run `wordcraft-cli --version` **on the target**; parse through the same
   `VersionProbe` acceptance as the local probe. Below-floor version or a binary that fails to
   start → `unsupported`/`error` with the concrete reason (ticket 09's verbatim-record rule).

Why controller-push, not target-download: ticket 06 fixes the transfer "over the workspace
transport"; it also keeps outbound network off the target (jump hosts, air-gapped boxes),
makes `SHA256SUMS.txt` verification one trusted step on the controller, and reuses transport
auth we already have. Cost: 82–100 MB through the tunnel, one time per host — mitigated by
the downloads cache, the remote sha short-circuit, and §2's per-host (not per-workspace)
install.

### 2. Install location: host-managed runtime root (rejected: workspace-scoped)

```
<remote-data-root>/wordcraft/<version>/     # extracted payload, immutable per version
<remote-data-root>/wordcraft/current        # pointer/marker to the active version
<remote-data-root>/wordcraft/profiles/<session-id>/   # isolated per-session profile (ticket 05 parity)
```

where `<remote-data-root>` = `$HOME/<hidden_data_directory()>/runtimes/wordcraft` resolved
once per connection via a remote `echo $HOME` probe (POSIX string joins only).

Rationale: the artifact is an arch-specific, workspace-independent *tool* — the same category
as the managed `node`/`python`/`pandoc` components — not workspace data. The flashgrep
workspace-scoped dir (`product_data_path(repo_root, ["bin"])`) fits a few-MB index tool bound
to one repo; repeating 100 MB per repository is the wrong default. Host-scoped also mirrors the
local layout so the resolution story is identical on both sides. Trade-offs accepted:
`$HOME` must be writable — if the probe shows otherwise, the record says so and the
**explicit-path resolution step** (below) is the documented escape hatch (point the remote
path override at a read-only-FS-safe location the operator chose). Version-scoped dirs +
`current` marker mean a pin bump is additive: old sessions keep their version, nothing is
deleted (root rule: destructive removal is an explicit user action — no idle cleanup of a
host install).

### 3. Resolution/discovery on the host — same three steps, run over the transport

`ManagedRuntimeResolver` itself stays local (it reads local paths); its *logic* is mirrored by
a remote probe executed over the connection, in the documented order:

1. **Explicit path**: configured remote wordcraft path for the workspace/connection (settings)
   → if set, verify it exists + `--version`-acceptable; if invalid → record `unsupported`
   citing that path (no silent continue to step 2 — an explicit config that fails is loud).
2. **System PATH**: remote `command -v wordcraft-cli` (operator-installed hosts).
3. **Managed**: `<remote-data-root>/wordcraft/current/…` (what §1 provisions).

Result feeds the same `WordcraftCapability { status, detail, version }` shape — **one record
vocabulary for local and remote**, so every surface answers identically (`available |
unsupported | not-installed | error`), with `detail` naming the target and the failed step.
Remote probe + provision live in `services-integrations` behind the `remote-ssh` feature; the
driving surface (desktop/web/bot/dispatch) never re-derives state.

### 4. Headless MCP over the workspace host: exec-channel stdio bridge (the real new work)

Local MCP stdio spawns via `process_manager` (`mcp/server/process.rs:143`) — that cannot run a
target-host binary. The remote session instead spawns `wordcraft-cli mcp --author …
--automation-*-root <workspace>` through the SSH **exec channel** (`open_exec_channel`) and
bridges its stdio JSON-RPC to the agent's MCP client — the same long-lived remote-stdio
session machinery flashgrep already uses (`RemoteStdioDaemonClient` / `RemoteStdioRepoSession`
with leases). Ticket 05's lifecycle carries over unchanged: one server per remote workspace
session, `WORDCRAFT_IDLE_REAP` (10 min) after last tool activity, `trap 'exit 130' HUP INT
TERM'`-style teardown on channel close so the remote process dies with the session. Session-id
and profile-dir validation reuse `WordcraftSessionSpec` verbatim (they are POSIX strings —
already host-agnostic). Remote roots are the remote workspace's POSIX path, normalized with
`normalize_remote_workspace_path` + `join_remote_path` + `shell_escape`; **no `std::path`
assembly for any remote string** (Windows-controller safe, per `product_paths.rs`'s contract).

### 5. Provisioning is a named, user-visible operation — never a hidden turn side effect

100 MB appearing mid-agent-turn would be an invisible stall (violates loud-degradation).
Provision runs as an explicit operation (capability-record "Provision on host" action +
workspace-connect offer, with progress), or is pre-warmed in the background **with a visible
notification**. If the agent hits a docx edit while the remote record is `not-installed`,
ticket 04's rule applies unchanged: the first step returns the capability record verbatim
("WordCraft is not available on `<host>` (not-installed)") — the agent does not edit locally,
does not convert through anydoc, does not pretend.

### 6. Acceptance states per target type

| Target | Provision + headless MCP | Live window | Notes / failure state |
|---|---|---|---|
| Direct SSH — Linux x86_64/aarch64 | **Supported** (full §1–§4) | Local-only; `ssh -L` + `WORDCRAFT_CONTROL_KEY` is the **documented** user path, never automated headless | Unknown distro without `tar` → `unsupported`, verbatim |
| SSH jump chain | Supported — probes, upload, and MCP spawn target the **final** workspace host only | n/a | Intermediate hops never receive the artifact |
| Docker/container workspace | Supported — `write_file` already routes to `container_write_file` | n/a | Ephemeral HOME → re-provision on recreate (idempotent §1); `not-installed` after recreate is expected and loud |
| WSL workspace | Supported via the Linux artifact through the existing WSL path | n/a | Same probes; distro env var for tests per services-integrations guide |
| macOS SSH host | Supported (universal CLI zip asset) | Local-only | Match on `--version` acceptance |
| Windows native SSH host | **Loud `unsupported` day one** | n/a | No trusted extraction tool guarantee (`tar`/`unzip` variance); revisit only with an explicit asset+tooling design — never a silent local fallback |
| Detached dispatch target | Supported **iff negotiated**: target capability ads must advertise the wordcraft headless capability (or provisioning support) before a docx-editing job is accepted — the `supports_provisioning` dispatch precedent (`dispatch_ssh.rs:1181`), never assumed | n/a | Missing ad → dispatch refuses up front with the concrete reason, not at first tool call |
| Remote control (mobile/IM bot) | Headless edits yes (rides the workspace host path); window gets the explicit unsupported reply (ticket 06.2) | **Explicit unsupported** | Registry rows already encode the command stance (ticket 15) |
| Controller offline (any target) | Record `error` (artifact fetch impossible), retryable | n/a | Keep record, degrade feature — never drop the session/workspace |

Remote version policy: below-floor (`< 0.7.0`-family floor from ticket 09) → `unsupported`;
**at-or-above floor is `available` with the actual version reported** — cross-version hosts
stay working (root upgrade rule: negotiate, don't assume; pin equality is the recommendation,
not a refusal), and pin-bump smoke (ticket 07's `list_commands`/`inspect_document` probe) runs
against the floor.

### 7. Scope fences

- `ssh -L` forwarding: documentation only (upstream precedent), never a headless transport.
- No target-side daemon: unlike the dispatch account daemon, the headless server is
  session-scoped (§4) — no persistent service to provision or deprovision on hosts.
- No registry/table changes beyond ticket 15's rows; remote capability ads for dispatch are
  protocol negotiation, not new Product Operations.
- Owner: `services-integrations` `remote-ssh` feature; `services-core::wordcraft` stays
  platform-neutral (record types only).

Verification obligation carried to ticket 16: this is a design answer — **no remote behavior
was exercised here**. Scenario evidence (loopback SSH fixture at minimum, real jump/container
where available) must come from 16's dispatch leg and a focused
`remote_ssh::wordcraft_provision` test group before any "supported" claim ships; local-only
tests are not evidence of remote behavior (root rule).

### Grilling round 2 amendments (2026-10-17, user decisions)

Two decisions amend the sections above:

1. **Provisioning trigger: automatic at session/task start (§5 amendment).** When an agent
   accepts a docx task against a remote workspace and the host record is `not-installed`, the
   session provisions immediately — download + verify happens as part of session start
   preparation, not only on an explicit user action. It stays **loud**: progress and outcome
   are user-visible (notification/record), and on failure the driving surface gets the record
   verbatim. What stays forbidden is a *silent* provision; visibility, not a manual gate,
   is the rule now.
2. **Process model: persistent single daemon on the host (§4 + §7 reversal).** The
   session-scoped spawn + 10-min idle reap is replaced by **one long-lived headless daemon per
   user per workspace on the host**, shared by later sessions. Consequences the implementation
   must carry:
   - Sessions **attach** to the daemon over the exec channel instead of spawning; attach must
     re-validate identity — same pinned version, same workspace roots (read/write), same
     author — and refuse loudly on mismatch instead of silently sharing another workspace's
     daemon.
   - The daemon **survives session closes and Alex restarts** (upgrade-compatibility
     rule: nothing user-reaching is killed to recover from a parse/connect failure). Its
     lifetime ends only by host reboot, an explicit stop action, or a version-bump install
     that the user confirms (old daemon explicitly stopped during upgrade start — never
     "delete state to recover").
   - The 10-min `WORDCRAFT_IDLE_REAP` policy **no longer applies** to remote-host daemons;
     desktop headless sessions keep their own reap policy (ticket 10) unchanged.
   - Attach is idempotent and reconnect-safe (root rule: resumable cursors, idempotent
     mutations — a re-attached session lands in the same document state the daemon holds).
   - Survives-disconnect review scope grows: the daemon owns document state while the
     controller is gone; every new command must survive rehydration.
   - Ticket 13's last-write-wins holds across daemon attach/detach.

Everything else in §1–§3 (probe → controller-fetch+verify → push → remote re-verify; host
root; resolution ladder) and §6's acceptance table stands.
