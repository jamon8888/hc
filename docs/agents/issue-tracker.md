# Issue tracker: Local Markdown

Issues and specs (you may know a spec as a PRD) for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- State is recorded in a `Status:` line near the top of each issue file. Two dialects are in use — see [Status dialects](#status-dialects) before writing one. Never mix dialects inside a single feature directory.
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

## Status dialects

Two dialects are in use across `.scratch/`. **Detect which one a feature directory already uses and keep writing it.** Do not convert a directory to the other dialect, and never mix them within one directory.

### Triage dialect

Used by `.scratch/basemind-integration/`.

- Format is `**Status:** <role>` and `**Blocked by:** NN (title), NN`
- `Status` carries one of the five canonical roles from `triage-labels.md`
- `Blocked by` is present on every ticket, reading `None (can start immediately)` when unblocked
- There is no `Type:` line
- Produced by `/triage` and `/to-tickets`

### Wayfinding dialect

Used by `.scratch/univer-cli-integration/`.

- Format is a backtick-wrapped `` `Status: <state>` `` and `` `Type: <kind>` ``, with an unbackticked `Blocked by: NN`
- `Status` carries a wayfinding state: `open`, `claimed`, or `resolved`
- `Blocked by` appears only when the ticket is genuinely blocked
- `Type` is present on every ticket: `research`, `prototype`, `grilling`, or `task`
- Produced by `/wayfinder`

### Choosing between them

Match whatever the feature directory already uses. For a new directory, pick the dialect of the skill that is producing the tickets: triage dialect for `/triage` and `/to-tickets`, wayfinding dialect for `/wayfinder`.

### Detecting a mismatch

`/wayfinder`'s frontier scan only recognises `open`, `claimed`, and `resolved`. Pointed at a triage-dialect directory it reports an empty backlog, which is a dialect mismatch rather than an absence of ready work. Likewise `/triage` finds no valid starting state in a wayfinding directory. Neither is a reason to rewrite the directory — switch to the dialect that directory uses, or start a new directory in your own dialect.

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` — the Notes / Decisions-so-far / Fog body.
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `claimed`/`resolved`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
