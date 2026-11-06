# 24: Remote, peer-device, and detached-dispatch closure

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: 21 (Single-writer persistence and recovery), 22 (Composer-driven live agent editing), 23 (Fallback and format-safe routing)

## What to build

Close the embedded surface against Alex's four remote scenarios without moving
remote document content to the controller or making a browser frame a hidden
runtime dependency.

## Acceptance criteria

- [ ] Remote-workspace open, command, and save operations execute through the
      workspace owner; controller-local fallback is impossible and remote POSIX
      paths remain remote strings.
- [ ] When the remote host cannot expose the embedded surface, Alex returns a loud
      unsupported state and preserves the supported host-side headless route.
- [ ] Peer Device Mode advertises the embedded capability from the Product Operation
      Registry and refuses controller-local operations on the wrong host.
- [ ] Mobile/bot remote control receives an explicit unsupported response when no
      visible embedded surface exists.
- [ ] Detached dispatch negotiates the headless WordCraft capability and does not
      depend on an attached browser frame or live controller connection.
- [ ] Remote, reconnect, capability-advertisement, and detached tests prove that no
      local content fallback occurs.
