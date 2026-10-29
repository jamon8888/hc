# Query-resolution: user term → entity alias → search

`Type: task`
`Status: resolved`

## Question

Design the pipeline that maps a user's natural-language query ("Jean Dupont's chronology") to an entity alias before RAG search, and re-tokenizes search hits back to session tokens for the LLM.

## Deliverable

A precise function spec (Rust + TS boundary) for:

1. **Composer interception** — in `useMessageSender.ts` / `agentAPI.startDialogTurn()`, extract candidate entity mentions from the user's raw prompt *before* pseudonymization.
2. **Alias lookup** — map each candidate mention to the workspace identity registry (value_hash → stable alias). If no alias exists, the term is not a known entity — pass through unchanged.
3. **Query rewrite** — replace the user's mention with the **unsalted stable alias** (`ENT_PERSON_a7f3`) in the query sent to `rag_search`.
4. **Hit re-tokenization** — `rag_search` receives hits with `DocEntity.text` (raw value). For each hit, map every `entity.text` via registry → stable alias → **apply per-session salt** → replace in `hit.text` before returning to LLM.
5. **Fallback** — if registry lookup fails (basemind down, alias missing), hard-fail the turn (D7).

## Resolves

Fog: query-resolution mechanic.

## Dependencies

- Identity registry store (ticket 03)
- `rag_search` tool (ticket 04, re-scoped from basemind-integration #06)
- Prompt pseudonymizer (ticket 05, re-scoped from basemind-integration #16)

## Answer

**Resolved 2026-10-16, revised after the same day's arbitrages** (raw index — D6
amended; opaque HMAC token — D3 clarified; `rag_search` exposed as a port —
`RagSearchPort` in `contracts/runtime-ports`, feature `rag-search`). Written into
corrected SPEC §4; implemented in `services/basemind-integration/src/query_resolution.rs`
(`QueryResolver`), consumed through the port by the `Tool` wrapper in `alex-core`.

Pipeline (all of it inside the port implementation; `session_id` comes from
`ToolUseContext`, never from the query text):

1. **Query entry** — `rewrite_query(query, session_id)` runs on the tool-call query.
   Composer-side mention extraction (deliverable 1) was **dropped**: the user's raw
   prompt never reaches this path — §11.4 pseudonymizes it at the Tauri interception,
   so what arrives here is session tokens plus ordinary search terms.
2. **Vault resolution** — load `SessionPayload { map, salt }` (missing or unreadable ⇒
   hard-fail, D7), then replace each echoed token with its raw value from the vault
   map (`[PERSON_9c21de04a7f3b2d1] → "Jean Dupont"`). The vault — not a salt-strip —
   is the only inverse: the token is an HMAC keyed by the session salt (D3) and
   decomposes into neither alias nor salt. Tokens with no vault entry pass through
   unchanged (already opaque, nothing leaks); plain terms pass through unchanged.
3. **Why raw** — the RAG index is raw (D6 amended): same trust boundary as the source
   files, so the resolved query must carry raw values to match anything. That is safe
   because the query stays local — loopback MCP to the sidecar — and everything
   *leaving* the pipeline is re-tokenized.
4. **Hit re-tokenization** — `retokenize_hits(hits, session_id)`: candidate values =
   `hit.entities[].text` ∪ a `redact_text` safety pass over the raw chunk (regex-class
   PII, NER-off scans), longest-first; each value → `value_hash` → registry alias
   (mint on first sight) → session token `[CAT_<HMAC16(salt, alias)>]` → replace in
   `hit.text`; upsert `token → value` into the session vault (D4). The workspace alias
   is never emitted into the tool payload — model context must not carry a
   cross-session identity (D3).
5. **Fallback** — unreachable basemind, missing session payload, or vault/registry
   persistence failure hard-fails the call (D7); raw text never leaves the pipeline.

**Deviations from the sketch above:** (a) no alias pass on the query — against a raw
index, aliasing a query value would *break* matching, and the query never leaves the
host; (b) a detection whose alias is missing is **minted** rather than hard-failing
(aliases are deterministic `ENT_<CAT>_<hash8>`; minting stores only an irreversible
hash), and "skip re-tokenization if missing" was rejected as I1-leaky — basemind-down
and vault-down remain hard failures; (c) output entities are `{category, token}` only,
no `alias` field.