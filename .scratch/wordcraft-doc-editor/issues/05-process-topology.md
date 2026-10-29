# process-topology

`Type: grilling`
`Status: resolved`

## Answer

Four topological decisions, all aligned with root AGENTS.md:

1. **Headless lifetime**: one `wordcraft-cli mcp` server **per workspace session with a document
   open**, reaped on idle (of the order of 10 minutes) and on session/workspace close. Matches
   ticket 04's live-if-open bridge (which needs a persistent process to connect to) and avoids
   per-turn doc-reload startup cost; spawning per tool call is rejected.
2. **Profile isolation + key handling**: every launched WordCraft process (visible app included)
   gets its own `WORDCRAFT_PROFILE_DIR` under Alex's state dir — isolated recovery copies,
   no fighting the user's own WordCraft prefs. The control key file is read **only** by
   Alex's desktop adapter for bridge auth; never logged (fresh empty per root LOGGING.md),
   never exposed into agent context.
3. **Confinement**: `--automation-read-root` = read root and `--automation-write-root` = write
   root mapped to the **active workspace folder, both**. Editing-in-place is the product need
   ("fix report.docx"); anything outside the workspace is refused by upstream's own gate with a
   readable error. Workspace switch = stop the session's server, next prompt re-resolves.
4. **Spawn policy (hard requirement)**: all WordCraft processes spawn exclusively through
   `alex_services_core::process_manager::{create_command, create_tokio_command}` — margins
   must exclude bare `std::process::Command`/`tokio::process::Command` — so Windows gets
   `CREATE_NO_WINDOW`; no console-window flashes. Discovery/order per ticket 01
   (explicit path → PATH → managed component).

Deferred to implementation (ticket 08) with ticket 07 as its validating probe: exact idle-reap
timer, server binary resolution cache lifetime, and what dispatch (06) changes about lifetimes.

