# 25: Real-device validation and rollout gate

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: 22 (Composer-driven live agent editing), 23 (Fallback and format-safe routing), 24 (Remote, peer-device, and detached-dispatch closure)

## What to build

Validate the complete embedded WordCraft experience on supported desktop hardware
and record the evidence needed before changing the default `.docx` experience.

## Acceptance criteria

- [ ] A real local `.docx` round trip proves open, visible edit, tracked changes,
      save acknowledgement, reopen, document inspection, and independent OOXML/text
      read-back.
- [ ] WebGPU and WebGL2 fallback are exercised on supported hardware, including
      keyboard focus, resize, paste, large-document transfer, frame reload, and
      startup failure.
- [ ] Native live-control and headless regression checks remain green, including
      outside-write-root refusal and agent authorship.
- [ ] Upgrade tests cover an older persisted session, compatible protocol drift,
      incompatible artifact, and safe native fallback.
- [ ] Remote-workspace, peer-device, and detached-dispatch evidence is recorded
      separately from local-only evidence.
- [ ] The rollout decision documents whether embedded `.docx` opening is enabled by
      default, what remediation is shown on failure, and how to revert to native
      WordCraft without data loss.
