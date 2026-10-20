# Re-scope basemind-integration tickets 15, 17, 18 for D2 (no local-model exemption)

`Type: task`
`Status: resolved`

## Question

D2 — "no PII to the LLM" is model-agnostic, local models included — invalidates the
local-model exemption three sibling tickets were built around. Re-scope them without
reopening D2: titles and dependency edges stay; the exemption disappears.

## Deliverable

1. **Ticket 15** (`15-local-model-detection.md`) — mark **superseded**: local model
   detection exists only to skip pseudonymization, so there is nothing to detect or
   build (`isLocalModelBaseUrl`, `shouldPseudonymize`, `is_local_model_base_url` are
   dead ends).
2. **Ticket 17** (`17-desktop-dialog-turn-interception.md`) — rewrite the interception:
   unconditional at `desktop_dialog_turn_request()`, the single master toggle as the
   only skip condition, fail-closed (D7), **no** rehydration map in request metadata
   (the original body stored `prompt_rehydration` in `metadata` — a D4 leak), restore
   and live-display rehydration at the render boundary only.
3. **Ticket 18** (`18-pseudonymization-settings-toggles.md`) — rewrite as one master
   toggle (`pseudonymize_prompts`, default `true`); delete
   `pseudonymize_local_models`; copy states the model-agnostic guarantee and that
   turning the toggle off explicitly voids I9; legacy config keys must still
   deserialize (upgrade compatibility).

## Resolves

Fog: — none; spawned directly by locked decision D2 (applied to the sibling effort).

## Dependencies

- D2 (locked), corrected SPEC §11.4/§11.7/§11.8
- `.scratch/basemind-integration/issues/16` (PromptPseudonymizer — predecessor of 17)

## Answer

**Resolved 2026-10-16.** All three files edited in `.scratch/basemind-integration/issues/`:

- **15 → `Status: superseded`** with a header note pointing at D2 and issue 09; its
  code sketch (TS + Rust local-endpoint detection) is explicitly *not* to be built.
- **17 rewritten** — no `should_pseudonymize`, no `is_local_model_base_url`, no
  `get_model_base_url` lookup; `message = pseudonymized.text` / `original_message` for
  UI; fail-closed hard error on basemind/registry/vault failure; the
  `metadata.insert("prompt_rehydration", …)` idea removed (map lives only in the
  session vault); restore-path `rehydrate()` as display projection, live RAG/tool
  results through the same render-boundary chokepoint. **Status: ready-for-agent.**
- **18 rewritten** — single switch `pseudonymize_prompts: bool` (default `true`),
  model-agnostic copy, legacy `pseudonymize_local_models` read-and-ignored for
  upgrade compatibility. **Status: ready-for-agent.**

Consistency: SPEC §11.3 already retires the local-model gate, §11.8 declares the one
master toggle, §13/§16 no longer list `localModelDetection`, and ticket 15 no longer
appears as a blocker of 17/18 (only ticket 14/16 do).
