# Answer — Detection: original value access + redaction control

**Investigator:** wayfinding map `.scratch/pii-safe-rag-spine/issues/02-detection-value-access.md`  
**Date:** 2026-10-15  
**Repo:** jamon8888/basemind fork at v0.33.0 (read-only)

---

## 1. Original value availability — **YES, fully available**

The detector runs on **raw text** and returns spans with the original substring.

### Pipeline trace

```
raw text (String)
       │
       ▼
xberg extract (for files) / identity (for inline text)
       │
       ▼
NER (GLiNER2, optional) + built-in regex patterns
       │
       ▼
DetectedSpan { start, end, label, confidence }     ← src/pii/pipeline.rs:197
       │
       ▼
translate_findings(scope, file_id, findings, chunks, detected_at, processed_by)
       │
       ├─► FindingInput { start, end, category, token }  ← token = positional "[PERSON_1]"
       │
       ▼
PiiEntity { entity_id, category, value_hash=hash(token), ... }  ← pii_lineage audit trail
```

### Where the original value lives

| Location | Struct | Field | Evidence |
|----------|--------|-------|----------|
| **Detection output** | `DetectedSpan` | `text` **NOT PRESENT** — but `start`+`end` index into the **original text the detector was given** | `src/pii/pipeline.rs:197` |
| **Redaction report** | `RedactionFinding` (xberg) | `text` via `original.get(start..end)` | `src/mcp/tools_redact.rs:329` |
| **MCP `redact_text` response** | `detections[i]` | `"text": "Jean Dupont"` (the actual matched substring) | `src/mcp/tools_redact.rs:320-338` |
| **MCP `redact_text` response** | `rehydration_map` | `Map<"[PERSON_1]" → "Jean Dupont">` — full reversible map | `src/mcp/tools_redact.rs:340` |
| **Search hit** | `DocEntity` (when NER on at scan) | `text: "Jean Dupont"` | `src/extract/doc.rs:162` |

**Conclusion:** The original value **is recoverable at every stage** — it is the raw input text sliced by `start..end`. `translate_findings` *discards* it by hashing the positional `token` instead, but the MCP `redact_text` tool **recovers it from the original text before that loss** and returns it in `detections` and `rehydration_map`.

---

## 2. Can index-time redaction be disabled? — **YES, via config**

`basemind.toml` `[documents]` section (schema in `src/config/v1.rs:24`, `DocumentsConfig`):

```toml
[documents]
enabled = true
embed = true                    # vector indexing on/off
redaction = {                   # redaction config
  enabled = true,               # ← DISABLE HERE to index raw text
  categories = [],              # empty = all categories
  strategy = "token-replace",   # token-replace | mask | hash | drop
  custom_terms = [],
  custom_patterns = [],
}
```

- `src/config/documents.rs`: `RedactionConfig { enabled: bool, ... }` → converted to `xberg::text::redaction::RedactionConfig` at `tools_redact.rs:311` via `basemind_config.to_xberg()`.
- In `scanner_docs.rs:345`, `translate_findings` is called **only if** `doc.redaction_findings` is non-empty — which comes from the extraction step that respects `redaction.enabled`.
- **If `redaction.enabled = false`**: documents are indexed **raw**, no findings produced, nothing written to `pii_lineage`.

**So `hc` CAN pre-redact itself and hand clean text to basemind** — just disable index-time redaction in config and feed already-redacted text to the scan daemon (or use `redact_text` with `file_path` for one-offs).

---

## 3. Independent detector call — **YES, via `redact_text` MCP tool**

```rust
// MCP tool
redact_text(text: "Jean Dupont lives at 123 Main St", categories: [], strategy: "token-replace")
// Returns:
{
  redacted_text: "[PERSON_1] lives at [ADDRESS_1]",
  rehydration_map: {"[PERSON_1]": "Jean Dupont", "[ADDRESS_1]": "123 Main St"},
  detections: [
    {category: "person", start: 0, end: 11, text: "Jean Dupont", confidence: 0.98},
    {category: "address", start: 17, end: 31, text: "123 Main St", confidence: 0.87}
  ],
  ner_ran: true
}
```

- `src/mcp/tools_redact.rs:72` — `redact_text` is an MCP tool, callable over HTTP or via the CLI shim `server.redact_text_cli()`.
- Per-call config: `categories` (filter), `strategy`, `custom_terms`, `custom_patterns`, `ner_model_dir`, `require_ner`.
- **Returns original value in `detections[i].text` and full `rehydration_map`**.

No lower-level "detect only" API — `redact_text` is the entry point, and it always redacts (with the chosen strategy). To just detect, use `strategy="token-replace"` and read `detections` + `rehydration_map`.

---

## 4. Category coverage

From `src/pii/patterns.rs` and `src/pii/pipeline.rs` (GLiNER2 labels):

| Source | Categories |
|--------|------------|
| **Built-in regex** | `email`, `phone`, `iban`, `credit_card`, `ipv4`, `ipv6`, `uuid`, `jwt`, `db_connection_string`, `api_key`, `aws_key`, `github_token`, `slack_token`, `generic_secret`, `date`, `time`, `money`, `percent`, `ssn` (US), `nim` (FR), `bsn` (NL), `vat` (EU), `passport` (multi) |
| **GLiNER2 NER** (42 PII labels) | `person`, `organization`, `location`, `date`, `time`, `money`, `percent`, `email`, `phone`, `url`, `norp` (nationality/religious/political), `facility`, `gpe`, `loc`, `product`, `event`, `work_of_art`, `law`, `language`, `ordinal`, `cardinal`, `quantity`, ... plus **AI Act** labels: `ai_system`, `high_risk_ai`, `prohibited_ai`, `foundation_model`, `general_purpose_ai` |
| **Custom** | `--custom-term label,value` and `--custom-pattern label,regex` — merged into pattern engine per call |

**Configurable per call** — `RedactTextParams.categories: Vec<String>` filters which categories are redacted (empty = all). The pattern engine runs all patterns but only emits findings for selected categories.

---

## 5. Architectural implication for `hc`

### ✅ (a) `hc` CAN call detection on raw text and recover value+span+category

**Evidence:**
- `redact_text` MCP tool accepts raw `text` (inline) or `file_path` (extracted by xberg) — `tools_redact.rs:125-163`
- Returns `detections` with `"text": original.get(start..end)` — **the actual value** — `tools_redact.rs:329`
- Returns `rehydration_map: Map<token → original>` — **full reversible map** — `tools_redact.rs:340`
- Per-call category filtering, strategy selection, NER control — `RedactTextParams` at `tools_redact.rs:24-58`

**So the pipeline for `hc` is:**

```
User prompt / document chunk
         │
         ▼
hc calls basemind.redact_text(text, strategy="token-replace")
         │
         ▼
Gets back:
  - redacted_text (with [PERSON_1] tokens)
  - rehydration_map (token → "Jean Dupont")
  - detections (category, start, end, "Jean Dupont", confidence)
         │
         ▼
hc hashes "Jean Dupont" → value_hash
         │
         ▼
hc checks identity registry (value_hash → stable alias "ENT_PERSON_a7f3")
         │
         ├─► If new: mint alias, store in registry (vault-encrypted)
         │
         ▼
hc replaces [PERSON_1] with session token [PERSON_9c21de04a7f3b2d1] in redacted_text
         │
         ▼
Sends tokenized text to LLM
         │
         ▼
Later: RAG hit returns DocEntity with text="Jean Dupont"
         │
         ▼
hc maps "Jean Dupont" → value_hash → alias → HMAC(salt, alias) → [PERSON_9c21de04a7f3b2d1]
```

**No fork changes required.** The identity layer (value→alias, session-token HMAC, vault) lives entirely in `hc`'s `basemind-integration` crate. Basemind is used exactly as designed: detect+redact, index, search, vault-store.

---

## Implications for the two tickets

| Ticket | Re-scoped body |
|--------|----------------|
| **06** `rag_search` | Call `memory mode="documents"` → for each hit, read `hit.entities[].text` → map via registry → replace with session token in the chunk text returned to LLM |
| **16** `PromptPseudonymizer + SessionVault` | Now also owns the **identity registry** (value_hash → alias) and **session-token logic**; calls `redact_text` for detection, encrypts with the local `vault.rs` path keyed by the OS-keychain passphrase |

*(Both rows are superseded in detail by issue 05's re-scope — port-based
`rag_search`, raw index (D6 amended), `[CAT_<H16>]` HMAC tokens (D3 clarified) — the
shapes above remain the correct pipeline.)*

---

**Resolves fog:** pipeline shape, registry salting, ADR refinement.