# Map — PII-safe RAG & identity spine

`Type: map`
`Status: resolved`

## Destination

A corrected §4 (RAG search) and §11 (prompt pseudonymization) of `SPEC-basemind-integration.md` establishing one PII-safe spine: a cross-document, per-workspace **entity identity** layer owned by `basemind-integration`, through which prompts, documents, RAG queries, and RAG hits all pass so that **no PII ever reaches any LLM** (cloud *or* local), while the human user's view stays fully rehydrated. Reaching the destination = that spec text is written and the re-scoped tickets 06/16 plus the new identity-registry work are `ready-for-agent`.

## Notes

Domain: OpenBitFun (Rust workspace + React) integrating the basemind sidecar (`jamon8888/basemind`, pinned to **v0.33.1**) for PII detection, redaction, indexing, retrieval, and vault storage. Evidence (tickets 01/02, ADR 0001) was gathered at v0.33.0; v0.33.1 is a patch with zero `src/` delta (Linux ARM64 asset + `github_only` publish input, no schema advance, no re-index) — the version to bundle since 0.33.0 shipped no `aarch64-unknown-linux-gnu` binary. The upstream `Goldziher/basemind` v0.29.x line is a diverged numbering stream (lower numbers, later dates) and is **not** the integration target.

Skills every session should consult: `research`, `prototype`, `grilling`, `domain-modeling`. Read `GLOSSARY.md` for the canonical terms (redaction / pseudonymization / rehydration / entity identity) and `docs/adr/0001-*.md` for the identity architecture.

**Locked decisions** (settled while charting; do not re-litigate without reopening this map):

- **D1** — Deliverable is a corrected §4 + §11 of the existing spec, not a new spec.
- **D2** — Local models are pseudonymized too; "no PII to the LLM" is model-agnostic. *(Applied 2026-10-16: sibling tickets 15 → superseded, 17/18 re-scoped — [issue 09](issues/09-ticket-rescope-15-17-18.md).)*
- **D3** — Token = stable entity alias + per-session salt; the salt is private per session and never reaches the LLM. *(Clarified 2026-10-16: the token is an **opaque HMAC** — `[<CATEGORY>_<H16>]`, `H16 = HMAC-SHA256(salt, alias)` on 16 hex. Alias **and** salt are both hidden inside it; reversal goes only through the vault's `token → value` map, never by stripping a suffix. Rejects the `alias + salt` concatenation: it exposed the salt as plaintext, giving a confirmation oracle and a cross-session link via the workspace-stable alias.)*
- **D4** — Rehydration happens at the UI render boundary (single chokepoint); storage stays redacted. *(Amended with D6: "storage" = session transcript, vault, registry — tokenized/encrypted; the workspace corpus and the RAG index sit inside the local trust boundary and stay raw.)*
- **D5** — basemind is a sidecar (process boundary), not a submodule / crate dependency.
- **D6** — RAG chunks are re-tokenized to session tokens on the way out; the index stores unsalted stable aliases. *(Amended 2026-10-16: the index stores **nothing of ours** — it is **raw**, in the same trust boundary as the source files. The scan daemon ingests directly (no "index now" RPC, issue 01), so the boundary that matters is the LLM one: query direction resolves tokens → raw values through the vault map; output direction re-tokenizes value → hash → alias → session token. `alias + salt` in the old D6/D3 wording is superseded.)*
- **D7** — If basemind is down, hard-fail the turn; never send raw text.
- **D8** — The vault key lives in the OS keychain, passed over IPC, never argv.
- **D9** — The identity registry is per-workspace; no cross-workspace correlation.
- **D10** — The identity registry is a new, dedicated, vault-encrypted store owned by `basemind-integration`, not the fork's `pii_lineage`.
- **D11** — *(locked 2026-10-16)* `rag_search` is exposed as a **port**: `RagSearchPort` in `contracts/runtime-ports` (feature `rag-search`); `basemind-integration` implements it; `openbitfun-core` (assembly) implements the `Tool` trait against the port and must **not** depend on `basemind-integration`; the concrete port is injected at the host composition root through `ToolRuntimeHandles`, the same seam as `RemoteExecPort`/`TerminalPort`. Tool payload carries `{category, token}` only — never the workspace alias.

**Why a map exists at all:** basemind v0.33.0 (pin: v0.33.1, same code) provides *no* cross-document PII identity. Its `entity_id` is a per-file span id (`file:category:start:end`) and its `value_hash` is a hash of the positional redaction token, not of the value. `pii_lineage` is a GDPR audit trail of *where* PII was found, not a *who/what-is-this* graph. Search hits carry no entity references. The identity layer is therefore net-new product logic (D10).

## Decisions so far

- [01](issues/01-basemind-ipc-surface.md): **Basemind exposes MCP tools** — `redact_text` (detect+redact with original values + rehydration_map), `memory mode="documents"` (search with `DocEntity.text` when NER was on), `vault` (encrypt/decrypt maps). **No "index now" RPC**; scan daemon handles ingestion. Agent Comms layer is agent-loop only.
- [02](issues/02-detection-value-access.md): **Original value IS available** — `redact_text` returns `detections[].text` (raw match) and `rehydration_map`. Index-time redaction is configurable off via `basemind.toml [documents].redaction.enabled`. `hc` calls `redact_text` on raw text, gets value+span+category, builds registry, re-tokenizes. Pipeline: **hc detects → registry → mints session token → sends tokenized text to LLM**.
- [03](issues/03-query-resolution.md): **Query-resolution** — `rewrite_query` loads the session payload and resolves echoed tokens → raw values **through the vault map** (the index is raw, D6'; the HMAC token has no strip-the-salt inverse, D3); `retokenize_hits` unions hit entities with a `redact_text` safety pass, mints value → hash → alias → `[CAT_<HMAC16(salt, alias)>]`, and upserts `token → value` into the session vault (D4); alias never appears in the payload. **RESOLVED** (2026-10-16, revised after the arbitrages; SPEC §4 + `query_resolution.rs`).
- [04](issues/04-registry-salting.md): **Registry salting** — fresh ≥128-bit CSPRNG salt per session, encrypted in the session vault; token `[<CATEGORY>_<H16>]` = `HMAC-SHA256(salt, alias)` on 16 hex at prompt + hit re-tokenization; inverse only via the vault map (query rewrite and UI rehydration) — no salt-stripping anywhere. **RESOLVED** (2026-10-16, D3 clarified; SPEC §11.5/§11.6).
- [05](issues/05-ticket-rescope-06-16.md): **Re-scoped tickets** — basemind-integration #06 (rag_search, now port-based) and #16 (pseudonymizer+vault+registry, now HMAC) rewritten in place against the corrected architecture; both **ready-for-agent**. **RESOLVED** (2026-10-16, revised after the arbitrages).
- [06](issues/06-identity-registry.md): **Identity registry** — **RESOLVED** — `IdentityRegistry::new()` fetches passphrase from keychain; `compute_value_hash()` SHA-256; `get_or_create_alias()` returns `ENT_<CAT>_<hash8>`; `flush()` persists encrypted store.
- [07](issues/07-session-vault.md): **Session vault** — **RESOLVED** — `SessionVault::new()` fetches passphrase from keychain; `store/load/delete_session()`; `generate_salt()` 128-bit CSPRNG.
- [08](issues/08-os-keychain.md): **OS keychain** — **RESOLVED** — macOS via `security-framework` 2.11; Windows/Linux stubs. `get_or_create_vault_passphrase()` generates 256-bit passphrase on first run, stores in keychain, retrieves thereafter.
- [09](issues/09-ticket-rescope-15-17-18.md): **D2 applied to sibling tickets** — **RESOLVED** (2026-10-16) — ticket 15 (local model detection) → **superseded**; ticket 17 rewritten (unconditional interception, master toggle as the only skip, no rehydration map in metadata, fail-closed D7); ticket 18 rewritten (single `pseudonymize_prompts` toggle, legacy key read-and-ignored). Both 17/18 **ready-for-agent**.

## Not yet specified

<!-- fog: in-scope, not yet sharp enough to ticket. Graduates as 01 and 02 resolve. -->

- ~~Query-resolution mechanic~~ → ticket 03, resolved
- ~~Registry salting~~ → ticket 04, resolved
- ~~Ticket re-scope detail~~ → ticket 05, resolved
- ~~ADR refinement~~ → **amended, not just confirmed**: `docs/adr/0001` gained an *Amendments (2026-10-16)* section covering the raw-index trust boundary (D6'), the opaque HMAC token (D3'), the `RagSearchPort` exposure seam (D11), and the v0.33.1 pin. The earlier "no amendment needed" call predates the D3/D6 arbitrages.

## Delivered (2026-10-16)

- **SPEC corrected:** §4 (RAG search: three layers with `RagSearchPort`, vault-based query resolution against the raw index, mint-on-hit re-tokenization with `redact_text` safety pass, HMAC session tokens, no alias in payloads, D7 hard-fail) and §11 (model-agnostic pseudonymization: redact_text surface, unconditional interception with the master toggle as sole skip, `[CAT_<H16>]` HMAC token format, storage table, D4 rehydration, updated decision/remote tables). §14: IPC/token-format/vault questions closed, and the "Index alias ingestion (D6 end-to-end)" question **closed as resolved** (raw index, D6 amended). §1: invariant **I9** added to the table. §13/§16: local-model detection rows removed (D2), runtime-ports rows added, verification commands corrected to real package names. §17: pin **v0.33.1** recorded.
- **Tickets:** `.scratch/basemind-integration/issues/06` and `16` rewritten → `ready-for-agent`.
- **Code:** `identity_registry.rs`, `keychain.rs`, `session_vault.rs`, `query_resolution.rs`, `rag_search.rs`, `prompt_pseudonym.rs` compile (`cargo check -p openbitfun-basemind-integration`, only 2 stub warnings from sibling tickets remain).
- **Bugs found & fixed this pass:** keychain features were activated by nobody (D8 dead on arrival — now target-gated deps forwarded by `basemind`); prompt flow clobbered the session map on every turn (earlier turns lost rehydration); query rewrite replaced positional tokens that never exist in raw queries; rewritten `Tool` impl used a nonexistent `runtime_ports::agentic` path and a placeholder session id (moved to an inherent `search()`; assembly wrapper is ticket 06); query rewrite now de-salts echoed model tokens (D6); hit re-tokenization upserts the session map (D4) and mints instead of leaking or skipping.
- **Remaining (outside this map's destination):** assembly-side `rag_search_tool.rs` wrapper + ProductToolPlan registration (ticket 06), the `RagSearchPort` + `ToolRuntimeHandles` seam and host injection (ticket 06), the code-alignment items below, `rehydrate()` (ticket 16), tickets 17/18 implementation, and document-side registration via ticket 04 of the sibling effort.

## Delivered — second pass (2026-10-16, after the four user arbitrages)

Four rulings landed after the first pass and are now folded in everywhere: **port
placement** (D11), **raw index** (D6 amended), **opaque HMAC token** (D3 clarified),
**D2 applied to sibling tickets** (issue 09).

- **SPEC:** §4 rewritten around `RagSearchPort` + vault-based query resolution + HMAC
  re-tokenization + raw-index content paragraph; §11 blockquote/11.1/11.2/11.3/11.5/
  11.6/11.7/11.9 updated to the HMAC token and vault-only inversion; I9 row added to
  §1; §14 token-format + index-ingestion questions closed; §13/§16 table rows fixed;
  §17 pin/evidence note added.
- **Tickets:** 03/04/05 answers rewritten to the arbitrated design; new
  [09](issues/09-ticket-rescope-15-17-18.md); sibling 15 → superseded, 17/18 →
  rewritten, 06/16 bodies updated (port, vault resolution, HMAC, no-alias payload,
  `redact_text` safety pass, raw-index config).
- **Docs:** `GLOSSARY.md` (Redaction now excludes the at-rest index; Pseudonymization
  widened to every model-facing crossing and defined as HMAC-of-alias; Rehydration no
  longer claims "everything at rest"); `docs/adr/0001` Amendments section + version
  note.
- **Verification:** docs-only — `git diff --check` clean.

## Code alignment — for the parallel code session

The spec/tickets are now the contract; the compiled code in
`src/crates/services/basemind-integration/` still implements the **pre-arbitration**
design and must change (this map's session does not touch code):

1. **Token format:** replace `format!("{alias}_{salt}")` (and any `_{salt}` suffix
   logic) with `[<CATEGORY>_<H16>]`, `H16 = HMAC-SHA256(salt, alias)` truncated to 16
   hex — in `prompt_pseudonym.rs` **and** `query_resolution.rs::retokenize_hits`.
2. **Query path:** `rewrite_query` must resolve echoed tokens → raw values **through
   the vault map**; delete the `raw_query.replace(&format!("_{salt}"), "")` strip and
   the `redact_text` alias pass (they cannot work: the index is raw and the HMAC has
   no suffix).
3. **Payload:** drop the `alias` field from `RetokenizedHit`'s serialized output —
   model context must carry `{category, token}` only (D3); keep the alias internal.
4. **Safety pass:** union `hit.entities[].text` with a `redact_text` pass over each
   raw chunk before re-tokenizing (I1 must not depend on scan-time NER config).
5. **Index config:** ship scan config with `[documents].redaction.enabled = false`.
6. **Port:** implement `RagSearchPort` (`contracts/runtime-ports`, feature
   `rag-search`) over `RagSearchTool::search`; `openbitfun-core` gets only the trait.
7. **Ticket 15 fallout:** `local_model_detection.rs` (and any TS twin) is superseded —
   delete or leave unreferenced; no caller may branch on model provenance.
8. **Metadata:** never write the rehydration map into request metadata (old ticket 17
   sketch) — the vault is its only home.

### Code-alignment status (code session, uncommitted)

- **1–4, 7 — done.** New `src/token.rs` mints `[<CATEGORY>_<H16>]` via
  `hmac = "0.12"` (workspace dep, `dep:hmac` under `basemind`);
  `prompt_pseudonym.rs` and `query_resolution.rs` both mint through it.
  `rewrite_query` is now a sync vault-map reversal (salt-strip and `redact_text`
  alias passes deleted); `retokenize_hits` is async with a per-chunk
  `redact_text` safety pass unioned with `hit.entities[]` (longest-first);
  `RetokenizedEntity` is `{category, token}` — alias stays internal.
  `local_model_detection.rs` git-removed (no TS twin existed). Session-token
  terminology replaced "salted token" in docs/comments; `identity_registry.rs`,
  `session_vault.rs`, and `ipc.rs` already matched the spec — untouched.
- **6 — done (seam only).** `contracts/runtime-ports` gains feature `rag-search`
  with `RagSearchPort` + `RagSearchRequest`/`Response`/`Error` DTOs, a
  `ToolRuntimeHandles::with_rag_search_port` cfg'd slot (Debug contract test
  updated), and the `tool-runtime-handles` aggregate now includes `rag-search`.
  `openbitfun-basemind-integration` implements the port as
  `RagSearchPortAdapter` (Mutex over `RagSearchTool::search`); `openbitfun-core`
  gets only `tools-rag = ["openbitfun-runtime-ports/rag-search"]`. Boundary
  rules: runtime-ports `featureProfiles`, basemind consumer edge
  `['workspace-ports', 'rag-search']`, core forwarder
  `capabilityForwarder('tools-rag', 'rag-search')`, plus the
  `check-core-boundaries.test.mjs` fixtures. Wrapper tool + ProductToolPlan +
  desktop injection remain ticket 06.
- **5 — spec constraint, no code home yet.** The sidecar owns
  `basemind.toml`; no config-emission caller exists in this repo, so the
  `[documents].redaction.enabled = false` requirement stays documented in the
  SPEC/tickets until an emission path lands.
- **8 — no-op.** Nothing writes the rehydration map into request metadata
  today; the vault is already its only home.
- **Verification:** `cargo check -p openbitfun-basemind-integration` and
  `cargo check -p openbitfun-runtime-ports --no-default-features` green;
  `cargo test -p openbitfun-runtime-ports --features rag-search --lib` (9 pass)
  and the `tool-runtime-handles,web-search-port` Debug contract test pass;
  `pnpm run check:core-boundaries` passes; `check:core-boundaries:test`
  148/148; `check:repo-hygiene` passes; `cargo fmt --check` clean on both
  crates. **Known pre-existing failure unrelated to this work:**
  `cargo check -p openbitfun-core --no-default-features` fails with 38
  `LocaleId::FrFR` errors from commit `25a66fded` (i18n fr-FR generated-code
  drift) — reproducible without these changes.

## Out of scope

- The other 21 `basemind-integration` tickets — they live in the sibling triage-dialect effort `.scratch/basemind-integration/`; this map only corrects the PII spine (06, 16) and adds the identity layer.
- Modifying the basemind fork (D5 / D10 keep the fork clean; identity is product logic in `hc`).
- Cross-workspace entity correlation (D9).
- The local-model pseudonymization exemption — reversed by D2; no longer a thing.
