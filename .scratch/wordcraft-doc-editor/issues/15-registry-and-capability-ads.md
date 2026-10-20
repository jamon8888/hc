# registry-and-capability-ads

`Type: task`
`Status: resolved`

Blocked by: 09

## Question

Add Product Operation Registry rows for every new Tauri command the effort introduces (spawn
open document / new docx / settings recheck …) in
`src/crates/contracts/product-domains/src/remote_surface/table.rs` — declaring remote-workspace
stance and `ControllerLocal` flags per ticket 06; the desktop closure test and capability
generator must pass (`pnpm run check:core-boundaries`, plus `check:repo-hygiene` if file naming
bites). Where the surface is services-owned without new Tauri commands, record that in the
Answer so the drift is explicit.

## Answer

Exactly two Tauri commands exist for this effort; both rows landed with ticket 12 in
`remote_surface/table.rs`:

| Command | Remote workspace | Peer (cli_peer) | Notes |
|---|---|---|---|
| `wordcraft_open_document_command` | `LocalOnly` | `ControllerLocal` | `REFUSED` on the remote path — the visible egui window launches on the desktop host only (ticket 06: visible-window entry stays local, loud refusal elsewhere) |
| `wordcraft_status_command` | `LocalOnly` | `ControllerLocal` | `CLI_NOT_IMPLEMENTED` — capability probe is a desktop-host fact; other surfaces read the shared record via their own path |

"No new docx" is **not** a third command: the `file.new-wordcraft-document` context entry calls
`wordcraftAPI.openDocument({})` (empty path → WordCraft's own start screen), reusing
`wordcraft_open_document_command` (verified in `WordcraftDocumentCommands.ts:132`).

Surfaces without new Tauri commands (explicit, so drift stays visible):

- **Headless agent MCP session** (`mcp::server::wordcraft_session` in `services-integrations`):
  services-owned, spawned by the managed MCP registry, not a Product Operation — no row
  applicable; its remote stance is recorded by ticket 14 instead.
- **Live `--connect` bridge** (same module): desktop-adapter-driven, rides the same
  `wordcraft_open_document_command` lifecycle, no independent command.
- **Web UI adapter** (`WordcraftAPI.ts`): wraps the two commands above via `api.invoke`, adds no
  command of its own.

Verification: `pnpm run check:core-boundaries` passes (desktop closure test + capability
generator included) and `pnpm run check:repo-hygiene` passes (product identity audit, filename
rules) — both run green with these rows.
