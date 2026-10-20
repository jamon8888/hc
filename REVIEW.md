# REVIEW.md

What a reviewer looks for in a change to OpenBitFun, and how the finding gets
written up. It applies to human reviewers and to agent review runs (the
`code-review` skill, a fresh-context subagent pass, or any pre-PR review).

This file is a **standards source**: name it alongside `CONTRIBUTING.md` and
the nearest `AGENTS.md` when you run a Standards/Spec review.

It is short on purpose. Anything repeated here must be readable in a minute.
Anything that needs more room belongs in the nearest `AGENTS.md`.

## What failure looks like here

| Area | Failure looks like |
| --- | --- |
| Docs | It lies — a command, path, or behavior that no longer matches the code |
| Config, CI, deploy scripts | It misroutes — wrong trigger, path, feature flag, or target |
| Shared core | A host-specific API leaks in (`tauri::AppHandle`, bare `std::process::Command` under a GUI host) |
| Tauri commands | Not `snake_case`, a UI component calling Tauri directly, or an unstructured payload |
| i18n | Missing key, placeholder mismatch, or a Web UI locale catalog imported by a smaller surface |
| Persisted shapes | A field made required or narrowed — old data on disk cannot supply it |
| Remote, peer, dispatch | Host `std::path` semantics on a POSIX remote path, or a capability used without negotiating it |
| Logging | Non-English text or emoji |
| Layer boundaries | A dependency pointing up a layer, `tokio/full`, or `product-full` used as a test shortcut |
| Release, installer, packaging | A step a user cannot undo |

Ratchet baselines (`scripts/theme-color-governance-baseline*.json`,
`scripts/i18n-*-baseline.json`) are never raised to make a failing audit pass.
A baseline value moving **up** is an **Important** finding.

## Writing the finding

- **Evidence is a file and a line, quoted.** A guess from a symbol's name is
  not a finding; go read it or drop it.
- **Tag the severity:** **Important** (breaks or misleads), **Nit** (style or
  taste), **Pre-existing** (already in the code before this PR — say so, so it
  does not read as the author's fault).
- **Report the class, not the line.** Name the pattern and every instance of
  it, so one fix closes all of them.
- **Skip whatever tooling already enforces.** Lint, `cargo fmt`, and the repo
  checks are not review findings.
- **The summary starts with a count**, and says plainly when there is nothing
  to fix: `0 findings.` "Looks fine" without a count is not a summary.

Caps:

- **Nit cap: three per PR.** If a diff has more, say so in one line and stop.
  Noise costs the author another commit and another round.
- Only flag problems in the change. Pre-existing issues get one tagged line at
  most, and never a blocking one.

Paths to skip entirely: `**/dist/**`, `**/gen/**`, `src/generated/**`,
`src/web-ui/src/generated/**`, `target/`, and lockfiles unless the PR's purpose
is a dependency change. Review the source contract, not generated output.

## Convergence

An agent author and an agent reviewer can keep each other busy forever. These
rules exist to stop that.

- **Every finding is answered** — fixed, or resolved with a reason. Silence
  never closes a finding.
- **Round cap: three.** After three rounds of review and fix, stop reviewing
  and reconsider the approach. The fourth pass is almost always re-finding old
  bugs.
- **Re-reviews read the new diff only**, once the first pass is done.
- **Never re-post a finding already posted** on this PR.
- **Findings that only restate the diff** — restating what the code does rather
  than what is wrong with it — are noise; drop them.

## Fix it at the source

When the same finding shows up in a second PR, the fix is not a third comment.

| The finding is… | Fix it in… |
| --- | --- |
| A convention this repo follows | This file, or the nearest `AGENTS.md` |
| Something a machine could catch | A lint rule, an audit, or a `scripts/check-*.mjs` |
| An instruction that went stale | The doc line it contradicts |
| A bug that shipped | A failure mode in this table, so the class gets caught next time |

## What humans decide

Everything else is a machine's job. People are for:

- **Should this exist at all** — direction, scope, taste.
- **Risk** — auth, credentials, release and packaging, CI workflows, persisted
  data shapes, security boundaries.
- **Architecture** that sets a pattern others will copy.
- **Reading the evidence** — the command that ran, its output, the screenshot.
  If you cannot say what the checks would miss, ask for a real review instead
  of a quick approval.
- **Owning the outcome** — a person is accountable for every change that ships,
  whoever typed it.
