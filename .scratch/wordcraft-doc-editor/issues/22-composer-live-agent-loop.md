# 22: Composer-driven live agent editing

`Type: task`
`Status: open`
Triage label: ready-for-agent

Blocked by: 21 (Single-writer persistence and recovery)

## What to build

Make a prompt submitted from Alex's existing composer act on the currently visible
embedded WordCraft document, with reviewable tracked changes and verification.

## Acceptance criteria

- [ ] The active embedded document is attached to the agent turn as structured
      document-session context without coupling the composer to WordCraft APIs.
- [ ] The WordCraft session route selects the embedded live owner and never starts a
      second headless writer for that document.
- [ ] Agent edits use explicit tracked-change enablement and author changes as
      `Alex Agent (<session>)`.
- [ ] Single commands and batches update the visible editor, return results through
      the agent channel, and save through the durable embedded owner.
- [ ] The agent can verify with command discovery, text retrieval, document
      inspection, and independent save/read-back behavior.
- [ ] Missing document context, closed frames, lost bridges, and unavailable
      capabilities return explicit errors and do not silently switch writers.
- [ ] Composer, session-routing, tracked-author, batch, save, and verification
      tests cover the complete prompt-to-document loop.
