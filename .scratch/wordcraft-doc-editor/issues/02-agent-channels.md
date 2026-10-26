# wordcraft-agent-channels

`Type: research`
`Status: resolved`

## Question

What are the exact machine channels upstream WordCraft exposes for driving a document, and which
ones can Alex's agent use to read, edit, and verify a document on user prompt?

## Answer

Two channels, both command-oriented over the same ~270+ command id space (`engine.commands` /
`list_commands` enumerates every command with id, label, shortcut, params, enabled):

1. **MCP over stdio (headless)** — `wordcraft-cli mcp`. No window. Tools: `list_commands`,
   `execute` (one command), `batch`, `new_document`, `open_document` / `save_document`
   (docx/doc(open)/odt/rtf/html/md/tex/txt/json; save also pdf/png), `type_text`, `select_text`,
   `get_text` / `inspect_document` (verify without screenshots), `render_page`, `parity`.
   `--connect 127.0.0.1:PORT` bridges it to a *running app*, which additionally exposes
   `screenshot`, `click`, `key`, `ui_inspect`.
2. **JSON control protocol (loopback)** — `wordcraft --control 7981` (or
   `WORDCRAFT_CONTROL_PORT`). Line-delimited JSON on 127.0.0.1 only; methods `engine.execute`,
   `engine.commands`, `document.inspect`, `ui.*` (click/drag/key/text/screenshot/render/resize),
   plus any unknown method name is treated as a command id (`{"method":"format.bold"}`).

Relevant protocol facts:

- **Auth**: app writes a random 256-bit hex key to `<profile>/control-key.local-<port>` (mode
  0600, removed on window close, new per launch). Send top-level `"key"` per request or `auth`
  once per connection; unauthorized → `{"ok":false,"error":"unauthorized"}` + close. 30 s
  connect-timeout before first authorized request. Key is only ever forwarded to loopback hosts
  (remote requires ssh port-forward + `WORDCRAFT_CONTROL_KEY`).
- **Economics**: one JSON object per line, ≤1 MiB, one reply per line; invalid JSON → one error
  reply then close; a failed command leaves the document unchanged (good for agent retries).
- **Positions**: `{"story": "body"|{"part":n}, "path": [block, row, cell, block, …],
  "off": byteOffset}` — the agent can address selection precisely.
- **Tracked changes / authorship**: edits record under the session author (`"WordCraft User"`
  default); set via `--author "…"`/`--agent-name` flag or `file.setAuthor {"name": "…"}`. Tracking
  toggles with `review.trackChanges {"value": true}` — always pass explicit `value` (bare toggle
  can *turn off* tracking on a doc saved with it on).
- **Confinement**: `--automation-read-root DIR` / `--automation-write-root DIR` gate every
  path-taking file read/write (open/save/export/insert/mail merge/compare/...) to given folders;
  headless server only (`--connect` + flags is an error). Caveats that matter for the agent
  contract: commands with no path (e.g. `text.insert`, `execute`) still mutate whatever document
  the session has open, `get_text`/`inspect_document` return full document text, and Zotero
  tools talk to a local Zotero socket — the roots do *not* cover these.
- **Resources** (MCP): `wordcraft://document` (inspect) and `wordcraft://commands`.
- **Web app**: separate wasm build (`apps/wordcraft-web`, `trunk serve`, port 8771). Unverified
  for embedding (round-trip fidelity, control channel in browsers) — belongs to ticket 03.

Conclusion for the agent loop: the headless MCP server is the primary agent channel (Alex
already bridges MCP servers per docs/agent-tools.md, and inspect_document gives a
screenshot-free verification loop); the loopback control channel is the secondary channel for a
live, visible editor. Author naming + `inspect_document` are the user-facing "the agent touched
the document" story.
