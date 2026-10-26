# validation-loop

`Type: task`
`Status: resolved`

Blocked by: 10, 11, 12

## Question

Ship the verification story per ticket 02/06/07 contracts, choosing verification at the owner
(each contributor follows the nearest local `AGENTS.md`; no mono-suite shortcuts):

- docx round trip: agent edit → save → tracked changes preserved (OOXML assertions as in spike
  ticket 07) → open again in WordCraft and in an independent reader.
- agent prompt→edit→verify e2e inside tests (headless session; loud-unavailable state test).
- detached dispatch job editing a docx with no interactive host (capability ads checked, not
  assumed).
- Out-of-contract regression list: `pnpm run check:core-boundaries`,
  `pnpm run fmt:rs`, focused crate tests, no theme audit delta (ticket 03 decided zero tokens).
- Real-GPU validation of the visible app + control channel (key file, `engine.execute`,
  screenshot) on macOS/Windows devices — owed because the spike machine had no usable GPU
  surface; and the optional wordcraft-web embed live probe (same-origin static host) before
  committing that upgrade path.
- Durable docs: fold the architecture contract into the owning area docs (this map + ticket
  answers are the source material).

## Progress (validation run on the spike macOS machine)

Validated here (all green):

- `cargo test -p openbitfun-services-core --no-default-features --features wordcraft --lib wordcraft`
  → 6/6 (record + checksum). Found and fixed a real compile error first: the checksum `tests`
  submodule lacked `use std::path::PathBuf;` (`use super::*` only reaches the intermediate
  `checksum` module, which imports `std::path::Path`).
- `cargo test -p openbitfun-services-integrations --no-default-features --features mcp --lib`
  → 30/30 family (incl. 8/8 `wordcraft_session`). `unreachable-pub` warnings pre-existing.
- `cargo test -p openbitfun-desktop --lib wordcraft` → 4/4 after fixing a second real bug:
  `wordcraft_session_profile_dir` seeded the fingerprint with stem+pid only, so two opens in
  one process derived the same SHA-256 and collided (start-screen included). Fixed by folding
  a monotonic per-process `AtomicU64` session sequence into the fingerprint parts.
- `pnpm run fmt:rs` clean; `pnpm run check:core-boundaries` green.
- `pnpm run i18n:audit`: pre-existing red (23 errors on a locales-free tree: fr-FR/flow-chat.json
  never tracked, fr-FR common.json 26 sections / 823 keys behind, shared-term duplicates over
  baseline). The wordcraft WIP contributed +6 missing fr-FR keys (openInWordcraft*,
  newWordcraftDocument* added to en-US/zh-CN/zh-TW without fr-FR) — fixed by adding the `file`
  section with the 6 French strings; fr-FR count is back to the 823 baseline (zero delta).
  The overall 22-vs-23 difference is the dynamic-key proof the WIP fixes (source references
  `file.openInWordcraft*`, locales supply them).

Owed (cannot run on this machine / out of local scope):

- 16.3 full detached-dispatch e2e (controller → remote target → docx edit, no interactive
  host): no two-host dispatch rig exists in-repo, and the remote-provisioning implementation
  it must exercise is design-only (ticket 14 resolved the design; the code has not landed).
  What *is* proven for this leg: the headless MCP session the dispatch target would run works
  with no interactive host at all (the 16.1 round trip spawns the bare binary in confinement),
  and the capability ads are registry rows validated by `check:core-boundaries`. The full
  dispatch e2e is therefore blocked on 14's implementation, not on validation methodology.
- 16.5 real-GPU visible-app leg (key file, `engine.execute`, screenshot) + the optional
  wordcraft-web embed live probe: owed to macOS/Windows GPU hardware per ticket 07 (this
  machine's Metal surface was unusable in the spike; SwiftShader only covers the embed probe).

## Answer

All locally runnable validation legs are green; the two hardware/implementation-dependent legs
are recorded as owed above with their owners (same carry pattern ticket 07 used for GPU).

- Real docx round trip (16.1): `node tests/e2e/scripts/wordcraft-doc-roundtrip.mjs --cli
  <spike-cached 0.7.0 binary> --session validation-loop --strict` → **PASS**: tracked
  insertions authored `Alex Agent (validation-loop)`, bold run present in OOXML,
  independent `text` read-back, outside-write-root refusal. No fake success anywhere.
- Agent e2e + loud-unavailable (16.2): covered by the round trip (headless session loop) plus
  the web-ui state tests — `FileTabManager.test.ts` "refuses a not-installed capability loudly
  without a code-editor dump" and `WordcraftDocumentCommands.test.ts` "surfaces launch
  failures loudly with the backend remediation message" (13/13 focused vitest).
- Regression list (16.4): `check:core-boundaries` green, `fmt:rs` clean, `check:web` green
  (tsc no-emit), crate tests green (services-core wordcraft 6/6, services-integrations mcp
  30/30, desktop wordcraft 4/4), i18n parity zero-delta vs the pre-existing red baseline.
- Two real bugs were found and fixed by this loop: the checksum test module's missing
  `PathBuf` import, and `wordcraft_session_profile_dir` deriving identical profile dirs for
  two opens in one process (folded a monotonic per-process sequence into the fingerprint).
- Durable docs (16.6): `docs/features/wordcraft-doc-editor.md` (+ zh-CN) already carry the
  verification section, the device-gate script, and the hardware-owed statement.

`Status: resolved`

### Supplement from the merged validation round

Harness contract details worth keeping (from the script authoring pass):

- Binary resolution order: `--cli` / `WORDCRAFT_CLI` / managed runtime tree
  (`<data>/runtimes/wordcraft/current…`) / system `command -v`; an absent or un-startable
  binary fails loud with remediation — the script cannot fake success.
- `--strict` upgrades the select/bold formatting leg to fatal; core legs (track → type → save →
  OOXML authorship assertion, independent read-back, outside-root refusal) are always fatal.
- En-route upstream-contract facts, now in the docs' agent-guidance section: the MCP surface is
  **16 tools**; dot-commands ride `execute {command, params}` / `batch {commands, keepGoing}`
  with ids from `list_commands` (`select.text {text, occurrence}`, `format.bold {value}` —
  MCP tool names like `select_text` do NOT work inside `batch`).
- Desktop loud-unavailable logic was extracted to a pure unit-testable decision,
  `wordcraft_api::visible_editor_rejection_reason` (launchable → None; CLI-only →
  visible-editor refusal that still admits agent editing; not-installed/error → record
  verbatim), giving `cargo test -p openbitfun-desktop --lib wordcraft` a 7/7 pass including
  the three new refusal-path tests.
