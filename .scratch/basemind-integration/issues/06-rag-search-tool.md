# 06: rag_search tool + registration

**What to build:** A `rag_search` tool exposed through **`RagSearchPort`** (`src/crates/contracts/runtime-ports/src/rag_search.rs`, feature `rag-search`): the `Tool` wrapper in `openbitfun-core` plus the port implementation in `basemind-integration` querying basemind's `memory mode="documents"` MCP tool and re-tokenizing hits to session tokens.

**Inputs:** `{ query: string, top_k?: number }`
**Outputs:** `[{ chunk: string, source_doc: string, score: number, entities: [{category, token}] }]`

**Blocked by:** 01, 02 (needs BasemindClient and document pipeline); PII spine tickets query-resolution (`.scratch/pii-safe-rag-spine/03`), registry salting (`04`), identity registry (`06`), session vault (`07`)

**Status:** ready-for-agent

**Design authority:** corrected SPEC §4 (PII-safe identity spine), ADR 0001 (+ its 2026-10-16 amendments), decisions D3/D4/D6/D7/D9/D10. Port placement, the raw index, and the HMAC token format were decided by the user on 2026-10-16.

**Three layers (decided 2026-10-16):**

1. **Port (TO BUILD — `contracts/runtime-ports`):** `RagSearchPort` trait + `RagSearchRequest { query, top_k, session_id }` / `RagSearchResponse` / error DTOs, behind feature `rag-search`; add `ToolRuntimeHandles::with_rag_search_port` + `rag_search_port()` accessor (the existing `ToolRuntimeHandles` Debug-format assertion test must be updated when the field is added).
2. **Tool wrapper (TO BUILD — `openbitfun-core`):** `src/crates/assembly/core/src/agentic/tools/implementations/rag_search_tool.rs` — `call_impl` parses `{ query, top_k }`, takes the **real** `session_id` from `ToolUseContext` (never a placeholder), fetches `context.runtime_handles.rag_search_port()`, delegates, returns `ToolResult::json(hits)`; no port injected ⇒ explicit "rag_search unavailable in this runtime" tool error, no fallback. **`openbitfun-core` must not depend on `basemind-integration`** — it only ever sees `Arc<dyn RagSearchPort>`.
3. **Pipeline (EXISTS — `services/basemind-integration`, must be adapted):** `rag_search.rs::RagSearchTool::search(query, top_k, session_id)` + `query_resolution.rs::QueryResolver`; the concrete `impl RagSearchPort` is built at the host composition root — beside where the desktop app constructs `PromptPseudonymizer` — and threaded into tool-context state the same way `RemoteExecPort` reaches tools.

**Implementation:**

0. **Tool wrapper + port** as in layers 1–2.
1. **Query resolution** — `QueryResolver::rewrite_query(query, session_id)`:
   - load `SessionPayload { map, salt }` (missing/unreadable ⇒ hard-fail, D7);
   - replace each token the model echoed back with its **raw value from the vault map** (`[PERSON_9c21de04a7f3b2d1] → "Jean Dupont"`): the index is raw (D6 amended) and the HMAC token has no strip-the-salt inverse (D3); tokens without a vault entry pass through unchanged (already opaque); ordinary terms pass through unchanged;
   - **delete** the old `_{salt}` strip and the `redact_text` alias pass — aliasing a query value would break matching against the raw index, and the query never leaves the host (loopback MCP).
2. **Search** — call basemind MCP `memory` with `mode="documents"`, the resolved `query`, `limit=top_k`.
3. **Hit re-tokenization** — for each `DocumentSearchHit`, on the **raw** chunk first:
   - candidates = `hit.entities[].text` (scan-time detection) **∪** `redact_text(hit.text).detections[]` (safety pass — regex-class PII and NER-off scans; I1 must not depend on index config), sorted longest-first (nested values, email inside a URL);
   - each value → `SHA-256` `value_hash` → `get_or_create_alias_for_value` (mint `ENT_<CAT>_<hash8>` on first sight) → session token `[CAT_<HMAC16(salt, alias)>]` (D3) → replace in `hit.text` (raw never leaves, I1);
   - upsert `token → value` into the session vault (D4 — UI rehydration of hit chunks);
   - return `hit.text` (now tokenized), `source_doc = hit.path`, `score = 1.0 - hit.distance`, `entities: [{category, token}]` — **no `alias` field**: the payload is model context, and a workspace-stable alias there would link this session to every other session of the workspace (D3);
   - basemind unreachable / session payload missing / safety pass failed ⇒ hard-fail (D7), never fall back to raw text.
4. **Registration:** `assembly/core/src/agentic/tools/registry.rs` behind `cfg(feature = "tools-rag")`, plus the `ProductToolPlan` row in Product Assembly so `product-full` materializes the tool.
5. **Index config:** the scan-daemon config we ship sets `[documents].redaction.enabled = false` — raw index, no basemind positional tokens mixed in (SPEC §14, D6 amended).

**Checklist:**

- [ ] Create `rag_search.rs` port module in `runtime-ports` (trait + DTOs + `rag-search` feature) and the `ToolRuntimeHandles` field/accessor; fix its Debug-format assertion test
- [ ] Create `rag_search_tool.rs` wrapper: `call_impl` + schema + real `session_id` from `ToolUseContext`; missing port ⇒ explicit unavailable error (never return raw `DocEntity.text` to the LLM)
- [ ] Add `impl RagSearchPort` in `basemind-integration` over `RagSearchTool::search`
- [ ] Rewrite `query_resolution.rs`: vault-map token → raw value; remove the salt-strip and the alias pass (D3/D6 amended)
- [ ] Retokenize path: entities ∪ `redact_text` safety pass, HMAC tokens, vault upsert, drop `alias` from the emitted payload
- [ ] If basemind, the session payload, or the safety pass is unavailable → hard-fail the call (D7); never fall back to raw text
- [ ] Register in `assembly/core/src/agentic/tools/registry.rs` under `cfg(feature = "tools-rag")`
- [ ] Add `tools-rag` feature to `assembly/core/Cargo.toml` + ProductToolPlan row
- [ ] Host injection at the composition root; `pnpm run check:core-boundaries` stays green (no `openbitfun-core` → `basemind-integration` edge)
- [ ] Verify tool appears in `get_all_tools()` and works in the agent loop
- [x] ~~Settle SPEC §14 open question "Index alias ingestion (D6 end-to-end)"~~ — settled 2026-10-16: **raw index** (D6 amended); prove it end-to-end (index carries no aliases/salt; hits leave tokenized)

**Feature flag:** `tools-rag` in `assembly/core/Cargo.toml` (gates tool *registration*; the concrete port arrives by injection, not by a basemind dependency).
