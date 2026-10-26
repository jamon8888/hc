# embed-viewer-feasibility

`Type: research`
`Status: resolved`

## Question

Can univer-cli's local Viewer be embedded inside the Alex Web UI (iframe or Tauri webview)? Determine from primary sources (univer-cli repo: `apps/cli/README.md`, Gateway/Viewer server code, headers it sends): does it emit CSP / `frame-ancestors` / X-Frame-Options that block framing; how is it authenticated; is the port stable or dynamic; does it support a base path or reverse proxy. If framing is blocked, what is the minimal supported alternative (external browser handoff) and does an official embedding path exist.

## Answer
Branch `research/embed-viewer-feasibility`, file `research/univer-cli/01-embed-viewer-feasibility.md`.

CAN_EMBED: the Viewer emits no CSP/`frame-ancestors`/X-Frame-Options, has no auth (binds 127.0.0.1, allow-all authz stub), fixed port (default 9123, read via `univer daemon status --json`), ships a `?mode=embedded&scope=…&editable=…` URL contract, and supports base-path/proxying (`/uf/*` HTTP+WS, prefix strip). Official guidance is same-origin proxy + `@univerjs-pro/cowork` (Pro package), not iframe; Tauri webview behavior needs an empirical check — handed to ticket 08.
