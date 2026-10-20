# Answer — Basemind IPC surface

**Investigator:** wayfinding map `.scratch/pii-safe-rag-spine/issues/01-basemind-ipc-surface.md`  
**Date:** 2026-10-15  
**Repo:** jamon8888/basemind fork at v0.33.0 (read-only)

**Re-verified 2026-10-20 against a live v0.34.0 daemon** (the pinned version). The
measured surface below still holds, and the live daemon serves exactly these MCP
tools: `redact_text`, `memory`, `vault`, `code`, `git`, `graph`, `shell`, `web`,
`admin`, `agents`, `workspace`. There is **no** `extract_and_redact_path` and **no**
`reveal_entity` tool — redaction is `redact_text` with a `file_path`, and the
reversible `rehydration_map` it returns is the only way back to an original value,
which is why the product owns the reveal (ADR 0001).

---

## Transport

The basemind sidecar exposes **two distinct transport layers**:

| Layer | Protocol | Entry | Methods |
|-------|----------|-------|---------|
| **Agent Comms** | msgpack over Unix socket (UDS) / named pipe (Windows) | `crates/basemind-agent-ipc/src/server.rs:152` `serve_connection()` | `AgentClient` trait: `next_event()` + `send_command(AgentCommand)` — only `UserMessage`, `Cancel`, `Shutdown`, `PermissionDecision`, `RoomPost`, `RoomLeave` |
| **MCP Server** | Streamable HTTP (rmcp 3.0) at `http://127.0.0.1:<port>/mcp?root=...&agent=...` + **in-process CLI shims** | `src/comms/http_frontend.rs`, `src/cli/redact.rs:93` `server.redact_text_cli()` | Named MCP tools: `redact_text`, `memory`, `vault`, `shell`, `registry`, `web`, `graph`, `git`, `admin`, `comms` |

**OpenBitFun connects as an MCP client** — the sidecar is started as a long-lived HTTP daemon (`basemind comms daemon`) and the UI talks to `http://127.0.0.1:<port>/mcp`. The CLI shims (`redact_text_cli`, `vault_cli`) are thin wrappers used by `basemind redact` and `basemind vault` commands; they call the *same* internal handlers the HTTP server routes to.

---

## Method Table (MCP tools only — Agent Comms is irrelevant for PII pipeline)

| Tool | Method | Request | Response | File:Line |
|------|--------|---------|----------|-----------|
| **redact_text** | `redact_text` | `RedactTextParams` — text/file_path, categories, strategy (token-replace\|mask\|hash\|drop), custom_terms, custom_patterns, ner_model_dir, require_ner | `{redacted_text, rehydration_map: Map<token→value>, detections: [{category,start,end,text,confidence?}], ner_ran: bool}` | `src/mcp/tools_redact.rs:72` |
| **memory** (documents mode) | `memory` with `mode="documents"` | `SearchDocumentsParams` — query, limit, scope, mime_type, overrides | `Vec<DocumentSearchHit>` — path, chunk_idx, text, mime_type, byte_start, byte_end, distance, rerank_score?, keywords?, **entities?: Vec<DocEntity>**, summary? | `src/mcp/tools_memory.rs:57`, `src/mcp/memory.rs:547` |
| **memory** (search mode) | `memory` with `mode="search"` | `MemoryParams` — query, limit, tag | Vector KNN hits over stored notes (not document chunks) | `src/mcp/tools_memory.rs:57` |
| **vault** | `vault` with `mode="encrypt\|decrypt\|find\|forget\|inspect"` | `VaultParams` — map (token→value), passphrase, query | `encrypt` → `encrypted_blob` (base64); `decrypt` → `map`; `find/forget` → matches; `inspect` → stats | `src/mcp/tools_vault.rs:40` |

---

## Detection

**Yes — `redact_text` returns the original value.**

- `detections` array at `tools_redact.rs:320-338`: each entry has `"text": original.get(start..end)` — the **actual matched substring** (e.g. "Jean Dupont", the IBAN string).
- `rehydration_map` at `tools_redact.rs:340`: `Map<token → original value>` — full reversible map.
- `DocEntity` at `src/extract/doc.rs:162`: `text: String` is the raw mention; also has `category`, `start`, `end`, `confidence?`.
- The detector runs on raw text via **xberg** (C library called through FFI). NER (GLiNER2) is optional and degrades to pattern-only.

---

## Redaction

**Yes — `redact_text` with `strategy="token-replace"` (default) returns `redacted_text` + `rehydration_map`.** Strategies: `token-replace` (reversible), `mask` (███), `hash` (sha256), `drop` (delete).

---

## Indexing

**No dedicated "index document" MCP tool.** Documents are indexed by the **scan daemon** (`src/scanner_docs.rs`) — a background process that watches the workspace, extracts files via xberg, runs redaction, writes doc blobs + LanceDB vectors + `pii_lineage` audit. The MCP layer **only queries** the already-built index (`memory mode="documents"`). There is no "index this text now" RPC.

---

## Search / Query

**Yes — `memory mode="documents"` returns `DocumentSearchHit` with `entities: Vec<DocEntity>`** (when NER ran at scan time).

`DocEntity` at `src/extract/doc.rs:162`:
```rust
pub struct DocEntity {
    pub category: String,      // "person", "email", etc.
    pub text: String,          // RAW VALUE — "Jean Dupont"
    pub start: u32,
    pub end: u32,
    pub confidence: Option<f32>,
}
```

So **search hits DO carry entity references with the original values** — but only for documents that were scanned with NER enabled. The `entities` field is empty otherwise.

---

## Vault

**Yes — exposed as MCP tool `vault`** (modes: `encrypt`, `decrypt`, `find`, `forget`, `inspect`). The CLI shim `server.vault_cli()` calls the same handler.  
**Encryption:** XPII wire format — magic + scrypt-derived AES-256-GCM. Passphrase is caller-supplied (CLI passes it as argv; HTTP caller puts it in JSON body).

---

## Lifecycle

| Aspect | Detail |
|--------|--------|
| **Spawn** | `basemind comms daemon` — binds singleton Unix socket / named pipe (the bind IS the lock) at `singleton::resolve_paths()` |
| **MCP HTTP** | Starts on loopback port (from env `BASEMIND_MCP_PORT` or auto-pick); requires `Authorization: Bearer <token>` where token is generated at daemon start and placed in env `BASEMIND_MCP_TOKEN` |
| **Config** | `basemind.toml` at `$XDG_CONFIG_HOME/basemind/basemind.toml` or `--config`; `[documents]` section controls embed, redaction, NER, reranker |
| **Shutdown** | SIGTERM / Ctrl-C / `Stop` RPC — 15s runtime drain timeout |

---

## MISSING (relative to corrected §4 / §11 needs)

| Need | Status | Workaround |
|------|--------|------------|
| **"Index this text now" RPC** | **MISSING** | Must run scan daemon or shell `basemind redact --file` + manually persist; no hot-index path |
| **Cross-document entity identity** | **MISSING** | `entity_id` = `file:category:start:end` (per-file span); no value-keyed alias. **Must be built in hc** (D10) |
| **Per-session salt application** | **MISSING** | `redact_text` returns positional tokens (`[PERSON_1]`). Salt + stable alias mapping must be applied by `hc` post-retrieval |
| **Query-resolution (user "Jean Dupont" → alias)** | **MISSING** | Must be built in `hc` — map user terms to registry aliases before search |
| **Vault passphrase via OS keychain** | **MISSING** | Currently argv / JSON body. `hc` must inject from keychain via IPC |

---

## Conclusion for the map

The **MCP tool surface is sufficient** for the corrected architecture:

- `redact_text` → gives detections with **original text** + `rehydration_map` → `hc` builds value→alias registry, salts tokens, stores map in vault
- `memory mode="documents"` → returns hits with `DocEntity.text` (raw value) when NER was on → `hc` re-tokenizes to session tokens at LLM boundary
- `vault` → encrypt/decrypt maps with caller-managed passphrase → `hc` injects passphrase from OS keychain

The **scan daemon** handles document ingestion; `hc` orchestrates the per-session pseudonymization layer on top. No fork changes needed.

---

**Resolves fog:** pipeline shape, query-resolution mechanic, ticket re-scope detail.