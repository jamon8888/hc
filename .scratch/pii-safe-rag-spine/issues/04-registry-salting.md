# Registry salting: per-session salt derivation & application

`Type: task`
`Status: resolved`

## Question

Define how the per-session salt is generated, stored, and applied to stable aliases so that:

- The same entity gets a **different token in every session** (cross-session correlation broken)
- The salt **never reaches the LLM** and is not persisted beyond the session
- Rehydration at the UI boundary can reverse it

## Deliverable

A precise spec for:

1. **Salt generation** — at session start (or session restore), generate a fresh **≥128-bit random salt** (CSPRNG). Store it **encrypted in the session vault** alongside the rehydration maps. Never derive from user content.
2. **Token format** — stable alias `ENT_PERSON_a7f3` + salt `sessX` → `[PERSON_a7f3_sessX]` (or similar). The prefix (`PERSON`) comes from the entity category; the suffix is the salt.
3. **Application points** —
   - **Prompt pseudonymization**: user's "Jean Dupont" → registry → `ENT_PERSON_a7f3` → apply salt → `[PERSON_a7f3_sessX]` sent to LLM
   - **RAG hit re-tokenization**: `DocEntity.text="Jean Dupont"` → registry → `ENT_PERSON_a7f3` → apply salt → replace in `hit.text`
   - **UI rehydration**: token `[PERSON_a7f3_sessX]` → strip salt → `ENT_PERSON_a7f3` → registry reverse lookup → `rehydration_map` → "Jean Dupont"
4. **Session restore** — on `RestoreSessionWithTurns`, decrypt the salt from the vault, re-apply to all historical turns for display.

## Resolves

Fog: registry salting.

## Dependencies

- Session vault (ticket 05)
- Identity registry (ticket 03)

## Answer

**Resolved 2026-10-16, with D3 clarified the same day** — the token is *not* the
`alias + salt` concatenation sketched above: it is an **opaque HMAC**. Written into
SPEC §11.5/§11.6 and §4; implemented in `session_vault.rs`, `prompt_pseudonym.rs`,
`query_resolution.rs`.

1. **Salt generation** — unchanged: `SessionVault::generate_salt()`, fresh ≥128-bit
   CSPRNG (OsRng, hex) on the first pseudonymization of a session, stored inside the
   encrypted `SessionPayload { map, salt }` protected by the OS-keychain passphrase
   (D8), never derived from user content.
2. **Token format** — `[<CATEGORY>_<H16>]`, where `H16` = first 16 hex characters of
   `HMAC-SHA256(key = session salt, message = workspace alias)`. Why not
   `alias + salt`: the concatenation exposes the salt as a plaintext suffix, so any
   observer holding model context can strip it, re-derive the workspace-stable alias,
   link the same entity across every session of the workspace, and — knowing both
   alias and salt — confirm a guessed value (a confirmation oracle). An HMAC keyed by
   a hidden salt admits none of that, and the token is reversible **only** through the
   vault's `token → value` map — there is no strip-the-salt arithmetic. basemind's
   positional `[PERSON_1]` remains transient, inside the `redact_text` response only.
3. **Application points** —
   - **Prompt pseudonymization:** raw value → registry alias → HMAC token sent to the
     LLM.
   - **RAG hit re-tokenization:** hit value → alias → the *same* HMAC token (fixed
     salt, fixed alias ⇒ same token across prompts, documents, and hits within the
     session); `token → value` upserted into the vault (D4).
   - **Query resolution — the inverse:** echoed tokens are resolved through the
     **vault map** back to raw values (the index is raw, D6 amended); no salt is
     stripped and no alias is derived from a token.
   - **UI rehydration:** `token → value` from the session vault at the render boundary
     (D4) — longest-token-first.
4. **Session restore** — `load_session` supplies salt + map; the prompt flow seeds the
   new map from the existing payload so later prompts never evict earlier turns'
   entries (bug found and fixed 2026-10-16).

**Correlation properties:** same entity ⇒ same token within a session (cross-turn
reference like "Jean Dupont's chronology" keeps working); different session salt ⇒
different token everywhere else; an observer of model context can neither link two
sessions (alias hidden), nor confirm a guessed value (no oracle without the salt),
nor recover the alias or salt from a token (HMAC is one-way without the key).