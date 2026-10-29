# 09: RevealableEntityBadge component

**What to build:** React component that renders a colored badge for PII entities with tooltip and click-to-reveal overlay. Never re-injects revealed value into markdown or conversation history.

**Blocked by:** 08 (BasemindAPI + sanitization)

**Status:** ready-for-agent

- [ ] Create `src/web-ui/src/app/components/panels/content-canvas/RevealableEntityBadge.tsx`:
  - Props: `entityType`, `entityId`, `maskedLabel` (e.g., `[PERSON_1]`)
  - Color by type: PERSON=blue, DATE=gray, ORG=green, IBAN=red, EMAIL=purple, PHONE=orange, ADDRESS=brown, etc.
  - Tooltip from `@alex/ui` showing entity type + count
  - On click → `basemindAPI.revealEntity(entityId)` → shows real value in temporary overlay (portal)
  - Overlay auto-dismisses on outside click / escape / timeout (5s)
  - Never modifies underlying markdown or conversation state
- [ ] Unit tests for rendering, click handling, overlay dismissal