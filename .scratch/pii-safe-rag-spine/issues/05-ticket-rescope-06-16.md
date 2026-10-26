# Re-scope basemind-integration tickets 06 & 16

`Type: task`
`Status: resolved`

## Question

Rewrite the bodies of the two existing tickets in `.scratch/basemind-integration/` to match the corrected PII spine architecture. The titles and dependency edges stay; only the implementation detail changes.

## Ticket 06 — `rag_search` tool + registration

**Current** (basemind-integration #06): Creates `RagSearchTool` calling `basemind.query_rag()`; returns hits with `entity_ids` (assumed stable).

**Re-scoped body:**

```markdown
# 06: rag_search tool + registration

**What to build:** A `rag_search` tool in `assembly/core/src/agentic/tools/implementations/rag_search_tool.rs` that queries basemind's `memory mode="documents"` MCP tool and re-tokenizes hits to session tokens.

**Inputs:** `{ query: string, top_k?: number }`
**Outputs:** `[{ chunk: string, source_doc: string, score: number, entities: [{category, alias, salted_token}] }]`

**Implementation:**
1. Call basemind MCP `memory` with `mode="documents"`, `query`, `limit=top_k`.
2. For each `DocumentSearchHit`:
   - For each `hit.entities[]` (DocEntity with `text`, `category`):
     - Hash `entity.text` → `value_hash`
     - Look up `value_hash` in identity registry → stable alias `ENT_<CAT>_<hash>`
     - If missing: skip re-tokenization for this entity (entity not yet known to registry)
     - Apply per-session salt → `[CAT_<alias>_<salt>]`
     - Replace `entity.text` in `hit.text` with salted token
   - Return `hit.text` (now salted), `source_doc=hit.path`, `score=1.0 - hit.distance`, `entities[]` metadata
3. Register in `assembly/core/src/agentic/tools/registry.rs` behind feature `tools-rag`.

**Feature flag:** `tools-rag` in `assembly/core/Cargo.toml`.

**Depends on:** Identity registry (ticket 03), Registry salting (ticket 04), Session vault (ticket 05).
```

---

## Ticket 16 — `PromptPseudonymizer + SessionVault`

**Current** (basemind-integration #16): Builds `PromptPseudonymizer` using `basemind::vault` (nonexistent) and positional tokens.

**Re-scoped body:**

```markdown
# 16: PromptPseudonymizer + SessionVault + Identity Registry

**What to build:** Three components in `basemind-integration/src/`:
- `identity_registry.rs` — per-workspace, vault-encrypted store: `value_hash → stable_alias`
- `prompt_pseudonym.rs` — pseudonymizes user prompts via basemind `redact_text` + registry + salting
- `session_vault.rs` — encrypts/decrypts rehydration maps + session salt with OS-keychain passphrase

**PromptPseudonymizer flow:**
1. Input: raw user prompt, session_id
2. Call basemind MCP `redact_text(text=prompt, strategy="token-replace")` → gets `rehydration_map` (positional tokens → values) + `detections` (category, start, end, value)
3. For each detection:
   - Hash the `value` → `value_hash`
   - **Identity registry**: `value_hash` → stable alias `ENT_<CAT>_<hash>` (create if new)
   - Apply per-session salt → salted token `[CAT_<alias>_<sessX>]`
   - Replace positional token in `rehydration_map` with salted token
4. Build **session rehydration map**: `salted_token → original value`
5. Store session rehydration map in `SessionVault` (encrypted with OS-keychain passphrase via IPC)
6. Return pseudonymized prompt (with salted tokens) + session_id for later rehydration

**SessionVault:**
- `encrypt_and_store(session_id, map)` — passphrase from OS keychain (via basemind IPC `vault mode="encrypt"`)
- `decrypt_and_load(session_id)` — for UI rehydration
- `delete(session_id)` — on session expiry

**Identity Registry:**
- `get_or_create_alias(value_hash, category)` → `stable_alias`
- `resolve_alias(alias)` → `value_hash` (for rehydration)
- Persisted as encrypted file in workspace data dir; key from OS keychain

**Feature flags:** `basemind`, `tools-rag`, `pseudonymization` in `basemind-integration/Cargo.toml`.

**Depends on:** Registry store (ticket 03), Registry salting (ticket 04), basemind IPC surface (ticket 01), Detection value access (ticket 02).
```

## Resolves

Fog: ticket re-scope detail.

## Dependencies

- Identity registry (new ticket)
- Registry salting (ticket 04)
- Session vault (ticket 05)
- Query-resolution (ticket 03)

## Answer

**Resolved 2026-10-16.** Both bodies rewritten in `.scratch/basemind-integration/issues/`:

- **06 `rag_search tool + registration`** — three-layer split (port in runtime-ports + `alex-core` `Tool` wrapper + `basemind-integration` pipeline, the pipeline itself already implemented), vault-based query resolution against the raw index, mint-on-hit re-tokenization with session-vault upsert, D7 hard-fail, `tools-rag` + ProductToolPlan registration. **Status: ready-for-agent.**
- **16 `PromptPseudonymizer + SessionVault + Identity Registry`** — redact_text → registry alias → HMAC session token flow, the three components + keychain (all compiling), seeded session map, `rehydrate()` gap and tests listed as remaining. **Status: ready-for-agent.**

**Corrections vs the draft bodies above:** token format is `[<CATEGORY>_<H16>]` with
`H16 = HMAC-SHA256(session salt, alias)` (D3 clarified — *not* the `alias + salt`
concatenation sketched here); unresolvable entities are **minted**, not skipped (I1);
the vault uses local `vault.rs` AES-GCM + scrypt keyed by the OS-keychain passphrase
instead of the non-existent `basemind::vault` path; query resolution resolves echoed
tokens through the **vault map** against the **raw** index (D6 amended) instead of
de-salting and aliasing — the index stores neither aliases nor salt; `rag_search` is
exposed as `RagSearchPort` (`contracts/runtime-ports`, feature `rag-search`), the
`Tool` wrapper lives in `alex-core`, and that crate must not depend on
`basemind-integration` (injection at the host composition root); the tool payload
carries `{category, token}` only — no workspace alias in model context. Side effects of
D2: ticket 15 is **superseded** and tickets 17/18 are re-scoped (issue 09).