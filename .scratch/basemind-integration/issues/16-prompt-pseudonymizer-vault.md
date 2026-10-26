# 16: PromptPseudonymizer + SessionVault + Identity Registry

**What to build:** Three components in `basemind-integration/src/`:

- `identity_registry.rs` — per-workspace, vault-encrypted store: `value_hash → stable_alias`
- `prompt_pseudonym.rs` — pseudonymizes user prompts via basemind `redact_text` + registry + salting
- `session_vault.rs` — encrypts/decrypts rehydration maps + session salt with OS-keychain passphrase

**Blocked by:** 02 (BasemindClient); PII spine tickets detection value access (`.scratch/pii-safe-rag-spine/02`), identity registry (`06`), session vault (`07`), OS keychain (`08`)

**Status:** ready-for-agent

**Design authority:** corrected SPEC §11 (model-agnostic), ADR 0001, decisions D2/D3/D4/D8/D9/D10.

**State:** the three components are implemented and compile (`cargo check -p alex-basemind-integration`). Remaining: `rehydrate()`, the interception wiring (ticket 17), and tests.

**PromptPseudonymizer flow (`prompt_pseudonym.rs` — implemented):**

1. Input: raw user prompt, session_id
2. Call basemind MCP `redact_text(text=prompt, strategy="token-replace")` → gets `rehydration_map` (positional tokens → values) + `detections` (category, start, end, value)
3. Load the session payload; if absent, mint a fresh **≥128-bit CSPRNG salt** (`SessionVault::generate_salt()`) — never derived from content (issue 04)
4. For each detection:
   - **Identity registry**: `get_or_create_alias_for_value(value, category)` → stable alias `ENT_<CAT>_<hash8>` (create if new)
   - Mint the session token `[CAT_<H16>]` with `H16 = HMAC-SHA256(session salt, alias)` truncated to 16 hex (D3 clarified — replaces the old `ENT_<CAT>_<hash8>_<salt>` concatenation: neither alias nor salt is readable off the token, and reversal goes only through the vault map)
   - Replace the positional token in `redacted_text` with the session token
5. Build the session rehydration map: seed it from the **existing** session payload (earlier turns keep rehydrating, D4), upsert this prompt's `token → original value` entries
6. Store `SessionPayload { map, salt }` in `SessionVault` (AES-GCM + scrypt via `vault.rs`, passphrase from OS keychain — D8, never argv) and `registry.flush()`
7. Return `PseudonymizedPrompt { text, rehydration_map (this prompt's new entries), session_id }`

**SessionVault (`session_vault.rs` — implemented):**

- `store_session(session_id, SessionPayload { map, salt })` — encrypted with the OS-keychain passphrase
- `load_session(session_id)` — salt + map for salting and UI rehydration
- `delete_session(session_id)` — on session expiry
- `generate_salt()` — 128-bit CSPRNG, hex-encoded

**Identity Registry (`identity_registry.rs` — implemented):**

- `get_or_create_alias(value_hash, category)` / `get_or_create_alias_for_value(value, category)` → `stable_alias`
- `resolve_alias(alias)` → `value_hash` (reverse lookup)
- `compute_value_hash(value)` — SHA-256 hex; `entry_count()` — fresh-mint detection
- `flush()` — persist as encrypted file in workspace data dir (D9/D10); key from OS keychain

**Keychain (`keychain.rs` — implemented):**

- `get_or_create_vault_passphrase()` — 256-bit passphrase generated on first run, stored in the OS keychain (macOS `security-framework`; Windows/Linux fail closed as stubs) — D8

**Checklist:**

- [x] `prompt_pseudonym.rs`: redact_text → registry alias → HMAC session token → session tokens (no positional `[PERSON_1]` tokens survive)
- [x] Session map seeded from vault so earlier turns keep rehydrating (D4)
- [x] `session_vault.rs`: encrypted rehydration map + salt per session; passphrase from OS keychain, never argv (D8)
- [x] `identity_registry.rs`: per-workspace encrypted `value_hash → alias` store (D9/D10)
- [x] `keychain.rs`: platform keychain feature actually active (target-gated deps forwarded by `basemind`)
- [x] Pseudonymization is model-agnostic — local models are pseudonymized too (D2)
- [x] **Swap the token construction to the D3 HMAC:** replace every `format!("{alias}_{salt}")` with `[CAT_<H16>]`, `H16 = HMAC-SHA256(salt, alias)` (16 hex) — in `prompt_pseudonym.rs` **and** in `query_resolution.rs` (`retokenize_hits`); rehydration/query-resolution go through the vault map only, never through stripping a suffix
- [ ] `rehydrate(text, session_id)` on `PromptPseudonymizer` — the single UI render chokepoint (D4): load session payload, replace `token → value`, longest-token-first
- [ ] Auto-cleanup on session delete (`delete_session` call sites)
- [ ] Unit tests: same entity → same token within a session, different token across sessions; no raw value in pseudonymized output; no alias or salt readable off a token (try stripping a suffix — must fail); history rehydration after later prompts

**Feature flags:** `basemind`, `tools-rag`, `pseudonymization` in `basemind-integration/Cargo.toml`.

## Comments

- 2026-10-16 (code session): D3 HMAC swap **delivered** in commit `95d2f37d5` — `src/token.rs::mint_session_token` mints `[CAT_<HMAC16(salt, alias)>]` (workspace `hmac = "0.12"`), used by both `prompt_pseudonym.rs` and `query_resolution.rs`. Query resolution goes through the vault map only (no suffix stripping); `retokenize_hits` runs a per-chunk `redact_text` safety pass unioned with scan-time entities (longest-first) and its payload carries `{category, token}` only — the alias never leaves the crate. Crate relocated to `src/crates/services/basemind-integration/`. Still open on this ticket: `rehydrate(text, session_id)`, session-delete auto-cleanup, and the unit-test list above.
