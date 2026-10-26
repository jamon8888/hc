# Spec — Alex + Basemind Integration (Cross-Platform Ready)

**Status:** Draft Technical Specification  
**Scope:** Complete integration of Basemind (jamon8888/basemind fork, PR in review) as the document extraction/redaction/RAG engine across all Alex surfaces (Desktop, CLI, Web UI, Mobile Web, Remote Connect). Models download locally on first use and are configurable in Settings.

---

## 1. Objectives & Invariants

| # | Invariant | Enforcement Point |
|---|-----------|-------------------|
| I1 | No unredacted text ever reaches the LLM context | Single interception at `convert_document_to_markdown` (§3) |
| I2 | Agent cannot read workspace outside Basemind pipeline | Replace `LocalWorkspaceFs` with `BasemindWorkspaceFs` decorator (§2) |
| I3 | Agent can never reveal original value of masked entity | `revealEntity` exists only in UI/Tauri layer, never as agent tool (§6) |
| I4 | User can view source AND redacted document | Dual-tab viewer (§7) |
| I5 | User sees count & types of detected PII | `RedactedEntity[]` from Basemind, aggregated in UI (§7) |
| I6 | Open workspace is queryable via natural language (RAG) | New `rag_search` tool (§4) |
| I7 | Works on every OS (Linux, macOS, Windows) with local model download | Basemind binary bundled; models lazy-download on first use (§8) |
| I8 | Models configurable in Settings (local paths, auto-download toggle) | Settings panel for Basemind (§9) |
| I9 | No PII from user prompts reaches any LLM — local models included (D2) | Single interception at `desktop_dialog_turn_request` (§11.4); fail-closed (D7); single master toggle that explicitly voids I9 when off (§11.8) |

**No constraint on LLM used (cloud or local)** — guarantees rest on what *reaches* the LLM.

---

## 2. Interception #1 — Workspace Filesystem Access

**File:** `src/crates/services/services-core/src/workspace.rs`  
**Target:** `LocalWorkspaceFs` (implements `WorkspaceFileSystem` from `alex_runtime_ports`)

All agent tools (`file_read_tool`, `ls_tool`, `grep_tool`, `file_write_tool`) go through this abstraction.

### 2.1 New Decorator: `BasemindWorkspaceFs`

**Create new crate:** `src/crates/services/basemind-integration/`

```
src/crates/services/basemind-integration/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # BasemindClient, extract_and_redact, reveal_entity, query_rag
│   ├── workspace_fs.rs           # BasemindWorkspaceFs (decorator)
│   ├── document_convert.rs       # basemind_extract_and_redact
│   ├── models.rs                 # Model management (download, verify, local paths)
│   └── ipc.rs                    # Local IPC to Basemind sidecar (stdio/Unix socket/TCP)
```

```rust
// workspace_fs.rs
pub struct BasemindWorkspaceFs {
    inner: LocalWorkspaceFs,       // Delegate metadata/exists/read_dir to upstream
    basemind: Arc<BasemindClient>,
}

impl BasemindWorkspaceFs {
    pub fn new(basemind: Arc<BasemindClient>) -> Self {
        Self { inner: LocalWorkspaceFs, basemind }
    }
}

#[async_trait]
impl WorkspaceFileSystem for BasemindWorkspaceFs {
    // Metadata ops: pure delegation (upstream changes inherited automatically)
    async fn metadata(&self, path: &str, follow_symlinks: bool) -> anyhow::Result<Option<WorkspaceMetadata>> {
        self.inner.metadata(path, follow_symlinks).await
    }
    async fn exists(&self, path: &str) -> anyhow::Result<bool> { self.inner.exists(path).await }
    async fn is_file(&self, path: &str) -> anyhow::Result<bool> { self.inner.is_file(path).await }
    async fn is_dir(&self, path: &str) -> anyhow::Result<bool> { self.inner.is_dir(path).await }
    async fn read_dir(&self, path: &str) -> anyhow::Result<Vec<WorkspaceDirEntry>> { self.inner.read_dir(path).await }
    async fn read_dir_bounded(&self, path: &str, max_entries: usize) -> anyhow::Result<Vec<WorkspaceDirEntry>> { 
        self.inner.read_dir_bounded(path, max_entries).await 
    }

    // Content reads: redirect to Basemind (redacted markdown)
    async fn read_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let redacted = self.basemind.extract_and_redact_path(path).await?;
        Ok(redacted.markdown.as_bytes().to_vec())
    }
    async fn read_file_text(&self, path: &str) -> anyhow::Result<String> {
        Ok(self.basemind.extract_and_redact_path(path).await?.markdown.to_string())
    }
    async fn read_file_bounded(&self, path: &str, max_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        let redacted = self.basemind.extract_and_redact_path(path).await?;
        let bytes = redacted.markdown.as_bytes();
        if bytes.len() > max_bytes { Ok(None) } else { Ok(Some(bytes.to_vec())) }
    }
    async fn read_file_text_bounded(&self, path: &str, max_bytes: usize) -> anyhow::Result<Option<String>> {
        self.read_file_bounded(path, max_bytes).await?.map(String::from_utf8).transpose().map_err(Into::into)
    }

    // Write ops: pure delegation (never redact on write)
    async fn write_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()> { self.inner.write_file(path, contents).await }
    // ... delegate all other mutating ops to inner
}
```

### 2.2 Injection Point

**Location:** `local_workspace_services()` in `workspace.rs` (line 366) and session assembly in `assembly/core`.

```rust
// In services-core/src/workspace.rs — add feature-gated constructor
#[cfg(feature = "basemind")]
pub fn basemind_workspace_services(workspace_root: String, basemind: Arc<BasemindClient>) -> WorkspaceServices {
    WorkspaceServices {
        fs: Arc::new(BasemindWorkspaceFs::new(basemind)),
        shell: Arc::new(LocalWorkspaceShell::new(workspace_root)),
    }
}
```

```rust
// In assembly/core — session creation, swap based on feature flag
#[cfg(feature = "basemind")]
let fs = basemind_integration::BasemindWorkspaceFs::new(basemind_client.clone());
#[cfg(not(feature = "basemind"))]
let fs = services_core::LocalWorkspaceFs;
```

**Feature flag:** Add `basemind` feature to `services-core/Cargo.toml` and `assembly/core/Cargo.toml`.

---

## 3. Interception #2 — Document Conversion (Single Convergence Point)

**File:** `src/crates/execution/tool-execution/src/fs/document.rs`  
**Function:** `convert_document_to_markdown` (line ~143)  
**Caller:** `file_read_tool.rs` line ~219 (only caller)

### 3.1 Extended Types

```rust
// In tool-execution/src/fs/document.rs (under feature = "document-read")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedDocument {
    pub markdown: Arc<str>,
    pub source_format: &'static str,
    pub redacted_entities: Vec<RedactedEntity>,   // ← NEW
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedEntity {
    pub entity_id: String,       // Stable ID, e.g. "p1", "iban_3"
    pub entity_type: String,     // "PERSON" | "ORG" | "DATE" | "IBAN" | "EMAIL" | ...
    pub occurrence_count: u32,
}
```

### 3.2 Replacement Function

```rust
// In basemind-integration/src/document_convert.rs
#[cfg(feature = "document-read")]
pub async fn basemind_extract_and_redact(
    bytes: Vec<u8>,
    path_hint: String,
) -> Result<ConvertedDocument, DocumentConversionError> {
    if bytes.len() > MAX_DOCUMENT_INPUT_BYTES {
        return Err(DocumentConversionError::new(
            "resourceLimit",
            format!("document is larger than the {} MiB Read limit", MAX_DOCUMENT_INPUT_BYTES / (1024 * 1024)),
        ));
    }

    let permit = document_conversion_semaphore().clone().acquire_owned().await
        .map_err(|_| DocumentConversionError::new("runtime", "conversion worker closed"))?;

    let result = basemind_client()
        .extract_and_redact(bytes, path_hint.clone())
        .await
        .map_err(|e| DocumentConversionError::new("runtime", e.to_string()))?;
    drop(permit);

    Ok(ConvertedDocument {
        markdown: Arc::from(result.markdown.as_str()),
        source_format: infer_source_format(&path_hint),
        redacted_entities: result.entities,
    })
}
```

### 3.2a Remote workspaces

The sidecar reads the file itself, so it can only redact a file the
controller can read. A workspace whose files live on an SSH host cannot be
redacted from the controller, and the interception is **fail-closed**: the
document read is refused rather than answered with unredacted text. Serving
those workspaces means running the sidecar on the remote host and proxying
the read (ticket 19).

### 3.3 Branch in Caller (file_read_tool.rs line ~219)

```rust
// In file_read_tool.rs — keep upstream line, add basemind branch
#[cfg(feature = "basemind")]
let conversion_call = basemind_integration::basemind_extract_and_redact(bytes, resolved_path.to_string());
#[cfg(not(feature = "basemind"))]
let conversion_call = convert_document_to_markdown(bytes, resolved_path.to_string());

let conversion = tokio::time::timeout(DOCUMENT_CONVERSION_TIMEOUT, conversion_call).await
```

**Preserve:** `DocumentCache` (key = sha256 + Format), `Semaphore` (concurrency bound), timeout wrapper.

---

## 4. New Tool — RAG Search on Open Workspace

> **Corrected — PII-safe identity spine.** The original §4 passed search hits straight
> through with `entity_ids: string[]`, assuming basemind exposed stable cross-document
> entity ids. It does not: basemind's `entity_id` is a per-file span id
> (`file:category:start:end`) and its `value_hash` hashes the *positional redaction
> token*, not the value. Cross-document identity is product logic owned by
> `basemind-integration` (ADR 0001, decisions D3/D4/D6/D7/D9/D10 in
> `.scratch/pii-safe-rag-spine/map.md`). Every hit is re-tokenized through the identity
> registry into an opaque session token (HMAC, D3 clarified) before it can reach the
> LLM. The RAG index itself stays **raw**, inside the same trust boundary as the source
> files it derives from (D6, amended) — the boundary that guarantees I1 is the one at
> the LLM, not one at rest.

**Ownership — three layers, one port:**

| Layer | Crate / file | Owns |
|---|---|---|
| Port | `src/crates/contracts/runtime-ports/src/rag_search.rs` (feature `rag-search`) | `RagSearchPort` trait + `RagSearchRequest` / `RagSearchResponse` / error DTOs — the only shape `alex-core` ever sees |
| Tool wrapper | `src/crates/assembly/core/src/agentic/tools/implementations/rag_search_tool.rs` (`alex-core`, feature `tools-rag`) | `Tool` impl (`call_impl`), input schema, the **real** `session_id` from `ToolUseContext`, delegation to `context.runtime_handles.rag_search_port()` |
| PII-safe search pipeline | `src/crates/services/basemind-integration/src/rag_search.rs` (`RagSearchTool::search`) + `query_resolution.rs` (`QueryResolver`) | `impl RagSearchPort`: query resolution, basemind MCP call, hit re-tokenization, vault upsert |

**Dependency rule:** `alex-runtime-ports` and `alex-core` must **not**
depend on `basemind-integration`. The concrete port is constructed at the host
composition root — beside where the desktop app already builds `PromptPseudonymizer`
in `AppState` — and reaches the tool through the existing `ToolRuntimeHandles` seam
(`with_rag_search_port`, mirroring how `RemoteExecPort` and `TerminalPort` are
threaded in). `alex-core` only ever holds `Arc<dyn RagSearchPort>`; if no port
is injected the tool reports itself unavailable (no silent local fallback).

**Pipeline** (ticket `.scratch/basemind-integration/issues/06-rag-search-tool.md`):

1. **Query resolution** (`QueryResolver::rewrite_query(query, session_id)`) — load
   `SessionPayload { map, salt }` for the session, then replace every token the model
   echoed back with its raw value from the vault map
   (`[PERSON_9c21de04a7f3b2d1] → "Jean Dupont"`). Model context only ever contains
   tokens (§11) while the index stores raw text (D6 amended), so **the vault — not a
   salt-strip — is what reverses a token**: the token is an HMAC (D3), its alias and
   salt are not recoverable from it. Tokens with no vault entry pass through unchanged
   (already opaque, nothing leaks); ordinary search terms pass through unchanged. The
   resolved query stays local — loopback MCP to the sidecar — the same boundary as the
   index itself. A missing or unreadable session payload hard-fails the call (D7).
2. **Search** — call basemind MCP `memory` with `mode="documents"`, the resolved
   `query`, `limit=top_k`.
3. **Hit re-tokenization** (`QueryResolver::retokenize_hits`) — for each
   `DocumentSearchHit`, on the **raw** chunk before anything else:
   - collect candidate values: `hit.entities[].text` (`DocEntity { category, text,
     start, end, confidence }` — raw values from scan-time detection) **union** the
     `detections[]` of a basemind `redact_text` pass over the chunk (catches
     regex-class PII and scans where NER was off — I1 must not depend on index
     configuration), sorted longest-first so a value nested in another (email inside
     a URL) is replaced outermost-first;
   - for each value: `value_hash = SHA-256(value)` → `get_or_create_alias_for_value`
     mints `ENT_<CAT>_<hash8>` on first sight, exactly like the prompt flow → mint the
     session token `[CAT_<HMAC16(salt, alias)>]` (D3) → replace the raw value inside
     `hit.text` before the hit can leave the pipeline (I1);
   - upsert each `token → value` pair into the session vault so the UI render boundary
     can rehydrate the chunk (D4);
   - return `{ chunk, source_doc, score, entities: [{ category, token }] }`. The
     workspace-stable alias is deliberately **absent** from the payload: the tool
     result is model context, and an alias visible there would link this session to
     every other session of the workspace (D3).
4. **Registration** — tool registry in `assembly/core/src/agentic/tools/registry.rs`
   behind `cfg(feature = "tools-rag")`, plus the `ProductToolPlan` row in Product
   Assembly so `product-full` materializes the tool.
5. **Hard failure (D7)** — if basemind is unreachable, or the session payload (salt +
   vault map) cannot be loaded, or the `redact_text` safety pass fails, the call fails
   with a clear error. No raw fallback.

**Input / output contract:**

```json
// input
{ "query": "[PERSON_9c21de04a7f3b2d1]'s chronology", "top_k": 8 }
// output
[{ "chunk": "…[PERSON_9c21de04a7f3b2d1] joined Acme in 2019…",
   "source_doc": "notes/meeting.md", "score": 0.82,
   "entities": [{ "category": "person", "token": "[PERSON_9c21de04a7f3b2d1]" }] }]
```

**Content indexed:** workspace files as they are on disk. basemind's scan daemon
indexes them directly and exposes no "index this text now" RPC (issue 01), so the
index is **raw** (D6 amended): the same trust boundary as the source files, with
`[documents].redaction.enabled` set to `false` so the index never mixes in basemind's
own positional tokens either. The registry's aliases and the session salt are never
written into the index; step 3 is the single chokepoint that keeps I1 true — at the
LLM boundary, not at rest.

---

## 5. Image/OCR Handling

**Decision point:** Basemind OCR capability.

| Option | Action |
|--------|--------|
| **A — Basemind has OCR** | Route `prepare_inline_image_attachments` (`attachments.rs`) and `image_processing.rs` through Basemind before any processing. |
| **B — No OCR yet** | Disable `ImageAnalysis` tool group (`ToolPackFeatureGroup::ImageAnalysis` in `execution/tool-provider-groups/src/lib.rs`) for this fork. Prevents silent leak via unredacted image path. |

**Recommendation:** Start with **Option B** (disable), migrate to A when Basemind OCR validated.

---

## 6. Entity Reveal (UI-Only, Never Agent)

### 6.1 Tauri Command

**File:** `src/apps/desktop/src/api/commands.rs` (near `read_file_content`)

```rust
#[derive(Deserialize)]
pub struct RevealEntityRequest {
    pub workspace_id: Option<String>,
    pub entity_id: String,
}

#[tauri::command]
pub async fn basemind_reveal_entity(
    state: State<'_, AppState>,
    request: RevealEntityRequest,
) -> Result<String, String> {
    basemind_client(&state)
        .reveal_entity(&request.entity_id)
        .await
        .map_err(|e| e.to_string())
}

// Also add for dual-tab viewer (§7.5)
#[derive(Deserialize)]
pub struct ReadRedactedDocumentRequest {
    pub workspace_id: Option<String>,
    pub file_path: String,
}

#[tauri::command]
pub async fn basemind_read_redacted_document(
    state: State<'_, AppState>,
    request: ReadRedactedDocumentRequest,
) -> Result<RedactedDocumentResponse, String> { ... }
```

**Register in `src/apps/desktop/src/lib.rs`:**
```rust
invoke_handler![
    // ... existing
    read_file_content,
    write_file_content,
    basemind_reveal_entity,      // ← add
    basemind_read_redacted_document, // ← add
]
```

**Structural guarantee (I3):** Command lives only in `apps/desktop/src/api/commands.rs` (UI layer via Tauri IPC). Never registered in agent tool registry (`assembly/core/src/agentic/tools/implementations/`). Malicious prompt cannot trigger it.

### 6.2 TypeScript Service API

**New file:** `src/web-ui/src/infrastructure/api/service-api/BasemindAPI.ts`

```ts
import { api } from './ApiClient';

export interface RevealEntityRequest {
  workspaceId?: string;
  entityId: string;
}

export interface RedactedDocumentResponse {
  markdown: string;
  entities: RedactedEntity[];
  sourceFormat: string;
}

class BasemindAPI {
  async revealEntity(entityId: string, workspaceId?: string): Promise<string> {
    return api.invoke<string>('basemind_reveal_entity', { request: { workspaceId, entityId } });
  }

  async readRedactedDocument(filePath: string, workspaceId?: string): Promise<RedactedDocumentResponse> {
    return api.invoke<RedactedDocumentResponse>('basemind_read_redacted_document', { 
      request: { workspaceId, filePath } 
    });
  }
}

export const basemindAPI = new BasemindAPI();
```

**Export in:** `src/web-ui/src/infrastructure/api/index.ts`

---

## 7. UI — Entity Rendering & Dual-Tab Viewer

### 7.1 Markdown Output from Basemind

```markdown
Le contrat a été signé par <span class="pii-entity" data-entity-type="PERSON" data-entity-id="p1">[PERSON_1]</span> le <span class="pii-entity" data-entity-type="DATE" data-entity-id="d1">[DATE_1]</span>.
```

### 7.2 Sanitization Schema Extension

**File:** `src/web-ui/src/infrastructure/markdown/MarkdownRenderer.tsx` (line ~259)

```ts
const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames || []), 'details', 'summary'],
  attributes: {
    ...defaultSchema.attributes,
    span: [...(defaultSchema.attributes?.span || []), 'className', 'data-entity-type', 'data-entity-id'],
  },
};
```

### 7.3 Custom Span Component

**In `components` memo (line ~1316):**

```tsx
span: ({ node, className, children, ...props }) => {
  const entityType = props['data-entity-type'] as string | undefined;
  const entityId = props['data-entity-id'] as string | undefined;
  if (entityType && entityId) {
    return (
      <RevealableEntityBadge
        entityType={entityType}
        entityId={entityId}
        maskedLabel={children}
      />
    );
  }
  return <span className={className} {...props}>{children}</span>;
}
```

### 7.4 `RevealableEntityBadge` Component (New File)

**File:** `src/web-ui/src/app/components/panels/content-canvas/RedactedDocumentTabs.tsx` (or shared UI components)

- Colored badge by type: PERSON=blue, DATE=gray, ORG=green, IBAN=red, EMAIL=purple, etc.
- Tooltip from `@alex/ui` on hover
- On click → `basemindAPI.revealEntity(entityId)` → shows real value **temporarily in overlay**, never re-injected into markdown or conversation history.

### 7.5 Dual-Tab Viewer: Source | Redacted

**Add to:** `src/web-ui/src/app/components/panels/content-canvas/types/content.ts`

```ts
export const FILE_VIEWER_TYPES: PanelContentType[] = [
  // ...existing
  'redacted-document-viewer',   // ← add
];
```

**New Component:** `src/web-ui/src/app/components/panels/content-canvas/RedactedDocumentTabs.tsx`

```
┌─────────────────────────────────────────────┐
│ [Source]   [Redacted (12 PII)]               │  ← Toolbar/ToolbarGroup (@alex/ui)
├───────────────────────────────────────────────┤
│  Source tab   → <PdfViewer path={source} />  (existing, pdfjs-dist)
│  Redacted tab → <MarkdownRenderer content={redactedMarkdown} />
│                 + Side panel: entities grouped by type
│                   e.g. PERSON: 5 · IBAN: 2 · DATE: 3 · ORG: 2
└─────────────────────────────────────────────┘
```

**Entity counts** derived from `redacted_entities: Vec<RedactedEntity>` (§3.1), aggregated client-side by `entity_type`.

### 7.6 Trigger on File Open

**In:** `FilesPanel.tsx` → `handleFileSelect` / `handleFileDoubleClick` (line ~912)

```tsx
// For supported document types (PDF, docx, etc.)
createTab({
  type: 'redacted-document-viewer',
  title: fileName,
  data: { sourcePath, redactedMarkdown, entities, sourceFormat },
});
```

**Two-phase load:** `PdfViewer` shows source immediately (direct disk read). Background call to `basemind_read_redacted_document` hydrates Redacted tab.

### 7.7 Guardrail

Source viewer (`PdfViewer`) is **visual only** — never used as text source for agent context (no ad-hoc OCR on raw PDF outside Basemind pipeline, per §5).

---

## 8. Basemind Sidecar & Cross-Platform Model Management

### 8.1 Basemind Binary Distribution

| Platform | Delivery |
|----------|----------|
| Linux (x86_64, aarch64) | Bundled in Tauri sidecar (`tauri.conf.json` → `bundle.sidecar`) |
| macOS (x86_64, arm64) | Bundled in Tauri sidecar, notarized |
| Windows (x86_64) | Bundled in Tauri sidecar (.exe) |
| Remote/SSH workspaces | Basemind runs on **remote host** (same binary, downloaded via SSH) |
| Mobile Web / IM Bots | Basemind runs on **desktop/CLI host** (controller proxies via Remote Connect) |
| Detached Dispatch | Basemind runs on **target host** (headless CLI profile) |

**Delivery** — the binary is downloaded from the pinned fork release at build
time and verified, rather than committed to the repository:

- `resources/basemind/VERSION.json` pins the release (`jamon8888/basemind`,
  `v0.34.0`) and, per platform, the SHA-256 and size of both the release
  archive and the binary inside it.
- `scripts/prepare-basemind-resource.mjs` downloads the archive for the
  build target, refuses an archive that does not match the pin, extracts the
  binary with the shared libraries it loads relative to itself, and verifies
  the binary against its own pin. A cached binary that no longer matches is
  repaired rather than trusted.
- `scripts/desktop-tauri-build.mjs` stages the extracted tree into the
  bundle as the `basemind` resource, the same way the Flashgrep daemon is
  staged.
- At startup the desktop resolves the bundled binary, starts the daemon
  with the fork's own `comms start` verb, and publishes the three ports
  below. When the sidecar is missing the PII surfaces refuse to run rather
  than fall back to unredacted text (D7).

**Ports** — the host owns the composition; the runtime only ever sees the
port traits in `alex-runtime-ports`:

| Port | Feature | Owns |
|------|---------|-------|
| `DocumentRedactionPort` | `document-redaction` | Interception #2: a document the agent reads is redacted before it reaches the model context (I1, §3) |
| `RagSearchPort` | `rag-search` | The `rag_search` tool's search seam (I6, §4) |
| `PromptPseudonymizationPort` | `prompt-pseudonymization` | Interception #3: the prompt the model sees carries session tokens (I9, §11.4) |

### 8.2 IPC Protocol

**Local:** stdio or Unix socket (Linux/macOS), named pipe (Windows)  
**Remote:** SSH tunnel + stdio on remote host

```rust
// basemind-integration/src/ipc.rs
pub enum BasemindTransport {
    LocalStdio { child: Child },
    LocalUnixSocket { path: PathBuf },
    LocalTcp { addr: SocketAddr },
    RemoteSsh { connection_id: String, remote_path: String },
}

impl BasemindClient {
    pub async fn spawn_local() -> Result<Self, BasemindError> { ... }
    pub async fn connect_remote(conn_id: &str, remote_path: &str) -> Result<Self, BasemindError> { ... }
    
    pub async fn extract_and_redact(&self, bytes: Vec<u8>, path_hint: String) -> Result<ExtractResult, BasemindError>;
    pub async fn extract_and_redact_path(&self, path: &str) -> Result<ExtractResult, BasemindError>;
    pub async fn reveal_entity(&self, entity_id: &str) -> Result<String, BasemindError>;
    pub async fn query_rag(&self, query: &str, top_k: usize) -> Result<Vec<RagHit>, BasemindError>;
    pub async fn ensure_models(&self, models: &[ModelSpec]) -> Result<(), BasemindError>;
}
```

### 8.3 Model Management (Lazy Download + Settings)

**Models:** PII detection (NER), embedding (RAG), optional OCR.

| Model | Purpose | Size | Download Trigger |
|-------|---------|------|------------------|
| `pii-multilingual` | NER for 50+ languages | ~150 MB | First document read |
| `embedding-minilm` | RAG vector index | ~90 MB | First `rag_search` or workspace index |
| `ocr-multilingual` | Image text extraction | ~200 MB | First image drop (if OCR enabled) |

**Model Spec:**
```rust
pub struct ModelSpec {
    pub name: String,
    pub version: String,
    pub download_url: String,      // GitHub Releases / HF Hub
    pub sha256: String,
    pub required: bool,            // false = optional (OCR)
}
```

**Settings Panel:** `src/web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx` → new "Basemind" section

- Toggle: "Enable Basemind document redaction"
- Toggle: "Auto-download models on first use"
- Model list with status (downloaded / not downloaded / updating)
- Manual "Download" / "Remove" buttons per model
- Local model directory override (advanced)

**Persistence:** Model metadata in `alex_core::service::config::GlobalConfig` → `basemind.models` section.

---

## 9. Settings Integration

### 9.1 Config Schema Extension

```rust
// alex_core::service::config::types
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BasemindConfig {
    pub enabled: bool,
    pub auto_download_models: bool,
    pub model_directory: Option<PathBuf>,  // None = default (app data dir)
    pub models: HashMap<String, ModelConfig>,
    pub sidecar_path: Option<PathBuf>,     // Override for custom builds
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub version: String,
    pub enabled: bool,
    pub local_path: Option<PathBuf>,
}
```

### 9.2 Settings UI

**File:** `src/web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx`

Add "Basemind" page with:
- Master enable/disable toggle
- Model management table (name, version, status, actions)
- Advanced: custom sidecar path, model directory
- Remote workspace: "Use remote Basemind" checkbox (runs on SSH host)

---

## 10. Remote Scenarios Compliance

| Scenario | Basemind Behavior |
|----------|-------------------|
| **Remote Workspace** | Basemind runs on SSH host. `BasemindClient::connect_remote()` tunnels stdio over SSH. File reads go through remote `BasemindWorkspaceFs`. |
| **Remote Control (Mobile/IM)** | Controller (mobile) sends `rag_search` / `Read` tools → executed on desktop host → Basemind runs locally on desktop. Reveal entity command proxied via Tauri IPC. |
| **Peer Device Mode** | Controller shell local; Basemind runs on peer host. `PeerHostCapability::Basemind` advertised from registry. Commands routed via `RemoteCommand` wire protocol. |
| **Detached Dispatch** | Target host runs headless (CLI profile). Basemind sidecar spawned by target. No controller connection required. Job payload includes model requirements. |

**Registry entry** (Product Operation Registry): `src/crates/contracts/product-domains/src/remote_surface/table.rs` — rows for `basemind_reveal_entity` and `basemind_read_redacted_document`.

The rows are **`LocalOnly` / `ControllerLocal` / `REFUSED`**, not `Supported`. The
sidecar runs on the controller and reads the file itself, so a workspace whose
files live on an SSH host is not redactable from here: the controller's
sidecar cannot read the peer's files, and the document is therefore refused
rather than read unredacted. Serving a remote workspace means running the
sidecar **on the remote host** and proxying the command, which is the remote
binary sync and the `RemoteCommand` proxy in ticket 19 — separate work with
its own version and checksum story. Until that lands, the refusal is the
honest stance, and the web UI states it rather than falling back to reading
the document unredacted.

---

## 11. User Prompt Pseudonymization (Model-Agnostic)

> **Corrected — PII-safe identity spine.** The original §11 had two defects:
> it **skipped pseudonymization for local models** (D2 reverses this — "no PII to the
> LLM" is model-agnostic), and it minted **positional** `[PERSON_1]` tokens per prompt
> (D3 clarifies this: the replacement is a session token minted as
> `HMAC(session salt, workspace alias)`, so an entity is the same token across prompts
> and documents within a session, a different token in every other session, and the
> token itself reveals neither the alias nor the salt). Design authority: `GLOSSARY.md`,
> `docs/adr/0001-*.md`, and the locked decisions D1–D10 in
> `.scratch/pii-safe-rag-spine/map.md`.

### 11.1 Objective

Pseudonymize **every** user prompt before it reaches the model — cloud *or* local —
through the workspace identity registry, keyed by a per-session salt, with rehydration
only at the UI render boundary.

**Invariant I9:** No PII from user prompts reaches any LLM, local models included (D2).
Session storage and the model-facing transcript stay tokenized; the workspace corpus
and the RAG index remain raw inside the local trust boundary (D6 amended); only the
human view is rehydrated (D4).

### 11.2 Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ USER PROMPT PSEUDONYMIZATION FLOW (model-agnostic)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│  Composer (ChatInput.tsx)                                                   │
│       │  raw prompt                                                         │
│       ▼                                                                     │
│  LocalSessionDriver.startTurn() → agentAPI.startDialogTurn()                │
│       │                                                                     │
│       ▼                    ┌──────────────────────────────────────────┐    │
│  Tauri: start_dialog_turn  │  PSEUDONYMIZATION INTERCEPTION POINT     │    │
│       │                    │  desktop_dialog_turn_request()           │    │
│       ▼                    │  1. basemind redact_text(prompt)         │    │
│  AgentRuntime.submit_      │     → detections[] {category, value}     │    │
│  dialog_turn()             │  2. value_hash → identity registry       │    │
│       │                    │     → stable alias (mint if new)         │    │
│       ▼                    │  3. token = HMAC(salt, alias)            │    │
│  Agent Loop                │  4. store token → value in SessionVault  │    │
│  (LLM sees ONLY            │     SessionVault (encrypted, keychain)   │    │
│   session tokens —         │  5. send pseudonymized prompt            │    │
│   cloud AND local)         │  6. any failure → HARD-FAIL turn (D7)    │    │
│                             └──────────────────────────────────────────┘    │
│  UI render boundary (single chokepoint, D4): history restore, live          │
│  display, RAG hits, tool results — all rehydrate here; everything           │
│  else (storage, index, model context) stays tokenized.                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 11.3 Detection surface — basemind `redact_text`

The original §11.3 ("local model detection" gate) is **retired**: pseudonymization has
no model-type branch (D2), so there is nothing to gate. `redact_text` is the single
detection entry point:

| Fact | Value |
|---|---|
| MCP tool | `redact_text(text, strategy="token-replace")` — `src/mcp/tools_redact.rs:72` in the basemind fork |
| Response | `redacted_text`, `rehydration_map` (`[PERSON_1] → "Jean Dupont"`), `detections[] = {category, start, end, text: original value, confidence}`, `ner_ran` |
| Categories | ~20 regex patterns (email, phone, iban, credit_card, ssn, …) + 42 GLiNER2 NER labels (person, organization, location, …), filterable per call |
| Transport | MCP Streamable HTTP on loopback (`Authorization: Bearer <token>`); `hc` connects as an MCP client (issue 01) |

**Pipeline (issue 02):** `hc` calls detection on the raw prompt → builds/looks up
`value_hash → alias` in the identity registry → mints the session token
`[CAT_<HMAC16(salt, alias)>]` (D3) → only tokenized text ever leaves the interception
point. basemind's positional `rehydration_map` is a transient input for this rewrite;
the durable map is the session vault's `token → value` map.

### 11.4 Interception point

**File:** `src/apps/desktop/src/api/agentic_api.rs` — `desktop_dialog_turn_request()`

Rules:

1. **Always run** when the `basemind` feature is enabled — there is no cloud/local
   branch anymore (D2). The old `should_pseudonymize` / `is_local_model_base_url`
   gate is deleted. The **only** bypass is the §11.8 master toggle being explicitly
   off (an accepted user override that voids I9 — §11.9); every other skip path is
   forbidden.
2. **Fail closed (D7):** if basemind is unreachable, the registry cannot be read, or
   the session vault cannot be written, the turn aborts with a clear error. Silent
   fallback to the raw prompt is forbidden.
3. `message` carries the pseudonymized text (what the LLM sees); `original_message`
   carries the raw text for the UI only (D4). The rehydration map must **not** be
   copied into request metadata or any model-visible field.

```rust
#[cfg(feature = "basemind")]
let pseudonymized = basemind_integration::get_prompt_pseudonymizer(&state)?
    .pseudonymize(&request.user_input, &request.session_id)
    .await
    .map_err(|e| format!("PII pseudonymization failed (basemind required): {e}"))?;

Ok(AgentDialogTurnRequest {
    session_id: request.session_id,
    message: pseudonymized.text,                  // session tokens only
    original_message: Some(request.user_input),   // UI display only (D4)
    // ...
})
```

### 11.5 PromptPseudonymizer

**File:** `basemind-integration/src/prompt_pseudonym.rs` (implemented).

```rust
pub struct PromptPseudonymizer {
    basemind_client: BasemindClient,
    registry: IdentityRegistry,   // per-workspace store (D9/D10)
    vault: SessionVault,          // passphrase from OS keychain (D8)
}
```

Flow of `pseudonymize(raw_prompt, session_id) -> PseudonymizedPrompt`:

1. `redact_text(raw_prompt, strategy="token-replace")` → `detections[]` (with the
   **original values**) + positional `rehydration_map`.
2. `vault.load_session(session_id)`; on miss, create the payload with a fresh
   **≥128-bit CSPRNG salt** (`SessionVault::generate_salt()`) — never derived from
   user content (issue 04).
3. For each detection:
   - `compute_value_hash(value)` (SHA-256);
   - `registry.get_or_create_alias_for_value(value, category)` → stable alias
     `ENT_<CATEGORY>_<hash8>` (minted on first sight, reused everywhere after);
   - mint the session token `token = [CATEGORY_<H16>]`, where `H16` is the first 16
     hex characters of `HMAC-SHA256(key = session salt, message = alias)` (D3);
   - record `positional_token → token` and `token → value`.
4. Replace the positional tokens in `redacted_text` with the session tokens.
5. `vault.store_session(session_id, SessionPayload { map, salt })` — encrypted with
   the keychain passphrase. The map is seeded from the existing session payload so
   entries written by earlier turns keep rehydrating (D4); the returned
   `rehydration_map` carries only this prompt's new entries.
6. `registry.flush()` — persist the encrypted per-workspace registry.
7. Return `PseudonymizedPrompt { text, rehydration_map, session_id }`.

**Token format (D3, clarified):** `[<CATEGORY>_<H16>]` — an HMAC of the workspace
alias under the per-session salt, truncated to 16 hex characters. The same entity is
the same token across every prompt, document, and RAG hit **within** the session, and
a different token in every other session, which breaks cross-session correlation. The
token reveals **neither** the alias nor the salt: it can be reversed only through the
session vault's `token → value` map — there is no strip-the-salt arithmetic — so an
observer holding model context cannot confirm a guessed value, cannot link sessions,
and cannot re-derive any workspace identity (issue 04). The alias and the salt exist
only in the registry and in the session vault — never in the index, never in model
context, never beyond the session.

**Positional tokens are never sent to the LLM** — basemind's `[PERSON_1]` form exists
only transiently between `redact_text` and step 4.

### 11.6 Storage — identity registry, session vault, keychain

| Store | Location | Protected by | Contents |
|---|---|---|---|
| Identity registry | `<workspace data dir>/identity_registry.enc` | AES-GCM + scrypt, keychain passphrase | `value_hash → ENT_<CAT>_<hash8>`; **per workspace** (D9), no raw values (D10) |
| Session vault | `<session dir>/<session_id>.vault` | AES-GCM + scrypt, keychain passphrase | `SessionPayload { token → value, salt }` |

**API:** `IdentityRegistry::{compute_value_hash, get_or_create_alias,
get_or_create_alias_for_value, resolve_alias, flush}` (files:
`identity_registry.rs`); `SessionVault::{store_session, load_session,
delete_session, generate_salt}` (file: `session_vault.rs`).

**Keychain (D8):** `keychain.rs::get_or_create_vault_passphrase()` generates a
256-bit passphrase on first run and stores it in the OS keychain
(macOS: `security-framework`; Windows DPAPI / Linux SecretService are stubs that
fail closed until implemented). The passphrase is read in-process — never argv,
never plaintext on disk. `keychain-macos|windows|linux` target deps are forwarded by
the crate's `basemind` feature.

### 11.7 History & UI rehydration (D4)

Rehydration happens **only at the UI render boundary** — the single chokepoint:

- session restore (`RestoreSessionWithTurns`), live turn display, RAG hit chunks,
  and tool results resolve tokens through `PromptPseudonymizer::rehydrate(text,
  session_id)` (implemented in ticket 16: load the session payload, replace
  `token → value`, longest-token-first);
- storage and the model-facing transcript stay tokenized — a transcript re-sent to
  the model is never rehydrated;
- §6 Entity Reveal remains UI/Tauri-only (I3).

```rust
// restore path — display projection only
for turn in turns.iter_mut() {
    if let Some(user_msg) = turn.user_message.as_mut() {
        user_msg.content = pseudonymizer
            .rehydrate(&user_msg.content, &session_id)
            .await
            .unwrap_or_else(|_| user_msg.content.clone());
    }
}
```

### 11.8 Settings UI

One master toggle; the local-model override is gone (D2):

```tsx
interface BasemindSettings {
  enabled: boolean;
  pseudonymize_prompts: boolean; // master toggle; turning it off explicitly voids I9
  auto_download_models: boolean;
}
```

Copy states plainly that prompts are pseudonymized for **every** model — local
models included — and that disabling the toggle sends raw prompts to the selected
model.

### 11.9 Decision matrix

| Scenario | Model | Pseudonymization | RAG |
|----------|-------|------------------|-----|
| User types "Jean Dupont lives at 123 Main St" | **Cloud** (OpenAI, Anthropic, …) | ✅ `[PERSON_9c21de04a7f3b2d1]` `[ADDRESS_4d8f02a1c6b7e905]` sent to the LLM | ✅ Available |
| Same prompt | **Local** (Ollama, LM Studio, vLLM) | ✅ Same pipeline, no exemption (D2) | ✅ Available |
| Document read via `Read` tool | Any | ✅ Always (via §3 redaction) | ✅ Available |
| Master toggle off | Any | ❌ Explicit user override; I9 no longer holds | ✅ Available |

### 11.10 Performance impact

| Pseudonymization | Added latency |
|------------------|---------------|
| Runs for **every** prompt (cloud and local) | ~15–40ms: PII detection + registry lookup/mint + vault write |

There is no skip branch anymore; the cost is unconditional and bounded by the
sidecar's detection latency.

### 11.11 Remote/Peer/Detached Scenarios

Interception lives in `desktop_dialog_turn_request()`, so pseudonymization runs
**wherever the turn executes** — the host that owns the session, registry, and
vault. The surface where the user types never bypasses it.

| Scenario | Behavior |
|----------|----------|
| **Remote Workspace** | Turn executes on the session-owning host; its sidecar, registry, and vault do the work. |
| **Peer Device Mode** | The peer host pseudonymizes at interception; the controller only ever submits raw input through the proxied command. |
| **Detached Dispatch** | The target pseudonymizes when the job's turn is submitted; the disconnected controller observes only tokens. |

Ticket 19 (remote-scenarios compliance) validates this table against the Product
Operation Registry rows.

---

## 12. Upgrade Compatibility

- **Persisted shapes:** Add `basemind` config section with defaults. Old configs work (disabled by default).
- **Cross-version:** `BasemindClient` advertises protocol version. Controller checks before using `reveal_entity` or `rag_search`.
- **Rename = migration:** If Basemind binary name changes, keep reading old name until no supported peer sends it.
- **Tests:** Legacy deserialization + old-payload round-trip tests in `basemind-integration/tests/upgrade_compat.rs`.

---

## 13. File Summary — Create / Modify

| File | Action | Section |
|------|--------|---------|
| `src/crates/services/basemind-integration/` | **New crate** (Cargo.toml, lib.rs, workspace_fs.rs, document_convert.rs, models.rs, ipc.rs, prompt_pseudonym.rs, session_vault.rs, identity_registry.rs, query_resolution.rs, rag_search.rs) | §2, §3, §8, §11 |
| `services-core/src/workspace.rs` | Add `basemind_workspace_services()`, feature flag | §2.2 |
| `tool-execution/src/fs/document.rs` | Add `RedactedEntity` to `ConvertedDocument` | §3.1 |
| `assembly/core/src/agentic/tools/implementations/file_read_tool.rs` | Branch conversion call (feature-gated) | §3.3 |
| `assembly/core/src/agentic/tools/implementations/rag_search_tool.rs` | **New tool** | §4 |
| `assembly/core/src/agentic/tools/registry.rs` | Register `rag_search` (feature-gated) | §4 |
| `execution/tool-provider-groups/src/lib.rs` | Decide `ImageAnalysis` fate (Option B: disable) | §5 |
| `apps/desktop/src/api/commands.rs` | Add `basemind_reveal_entity`, `basemind_read_redacted_document` | §6.1 |
| `apps/desktop/src/lib.rs` | Register new Tauri commands | §6.1 |
| `apps/desktop/src/api/agentic_api.rs` | Modify `desktop_dialog_turn_request()` for pseudonymization | §11.4 |
| `web-ui/src/infrastructure/api/service-api/BasemindAPI.ts` | **New service API** | §6.2 |
| `web-ui/src/infrastructure/api/index.ts` | Export `basemindAPI` | §6.2 |
| `web-ui/src/infrastructure/markdown/MarkdownRenderer.tsx` | Extend `sanitizeSchema`, add `span` custom renderer | §7.2, §7.3 |
| `web-ui/src/app/components/panels/content-canvas/RevealableEntityBadge.tsx` | **New component** | §7.4 |
| `web-ui/src/app/components/panels/content-canvas/types/content.ts` | Add `'redacted-document-viewer'` to `FILE_VIEWER_TYPES` | §7.5 |
| `web-ui/src/app/components/panels/content-canvas/RedactedDocumentTabs.tsx` | **New dual-tab viewer** | §7.5 |
| `web-ui/src/app/components/panels/FilesPanel.tsx` | Route document clicks to new viewer type | §7.6 |
| `web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx` | Add Basemind settings page (incl. pseudonymization toggles) | §9.2, §11.8 |
| `src/crates/contracts/runtime-ports/src/rag_search.rs` | **New port** — `RagSearchPort` + request/response DTOs (feature `rag-search`) | §4 |
| `src/crates/contracts/runtime-ports/src/tool_runtime_handles.rs` | Add `with_rag_search_port` / `rag_search_port()` accessor | §4 |
| `web-ui/src/flow_chat/hooks/useMessageSender.ts` | ~~Pass `session_id` context for detection~~ not needed — interception is backend-side and carries `session_id` itself (§11.4); local-model detection retired by D2 | §11.3, §11.4 |
| `src-tauri/tauri.conf.json` | Add Basemind sidecar binaries per platform | §8.1 |
| `Cargo.toml` (workspace) | Add `basemind-integration` to workspace members | — |

---

## 14. Open Questions / Decisions Needed

- [ ] **Basemind OCR ready?** Determines §5 Option A vs B.
- [ ] **Redact filenames in `read_dir`?** (§2.1) — currently only content redacted.
- [x] **Exact Basemind IPC protocol** — **resolved (issue 01):** MCP Streamable HTTP on
  loopback with bearer token (`http://127.0.0.1:<port>/mcp`); Alex is an MCP
  client. Agent Comms (msgpack/UDS) is agent-loop-only and irrelevant to the PII
  pipeline.
- [ ] **Cache strategy** — reuse `DocumentCache` or dedicated Basemind cache (redaction latency higher)?
- [ ] **Model download source** — GitHub Releases vs Hugging Face Hub vs self-hosted?
- [ ] **Remote Basemind binary sync** — how to ensure remote host has matching version?
- [ ] **PII detection scope for prompts** — only user text, or also attached context (file paths, @mentions)?
- [x] **Token format consistency** — **resolved (D3, clarified):** one format everywhere —
  `[<CATEGORY>_<H16>]`, where `H16` is `HMAC-SHA256(session salt, alias)` truncated to
  16 hex characters (§11.5, §4). The token hides both the alias and the salt and is
  reversed only through the session vault. Positional `[PERSON_1]` exists only
  transiently inside the `redact_text` response and is replaced before anything leaves
  the interception point (§11.5).
- [x] **Vault storage location** — **resolved (D8/D10, §11.6):** per-session encrypted
  file `<session_id>.vault` plus a per-workspace `identity_registry.enc`, both
  encrypted with a passphrase held in the OS keychain.
- [x] **Index ingestion (D6 end-to-end)** — **resolved (D6 amended):** the index stays
  **raw**, inside the same trust boundary as the source files. The scan daemon indexes
  workspace files independently and basemind exposes no "index this text now" RPC
  (issue 01), so there is no alias-substitution ingestion path to build — and none is
  wanted: `[documents].redaction.enabled = false` keeps basemind's own positional
  tokens out of the index, and hit re-tokenization at the LLM boundary (§4, step 3) is
  the single chokepoint that keeps I1 true. Query direction: tokens → raw values via
  the session vault (§4, step 1); output direction: value → alias → session token.
  Settled 2026-10-16; ticket 06 may prove end-to-end behavior.

---

## 15. Upstream Sync Strategy (from original §10)

**Principle:** Maximize additive code (new files), minimize & harden modifications.

### 15.1 Conflict Risk Classification

| File | Nature | Conflict Risk |
|------|--------|---------------|
| `rag_search_tool.rs` | New file | None |
| `BasemindAPI.ts` | New file | None |
| `RevealableEntityBadge.tsx` | New file | None |
| `RedactedDocumentTabs.tsx` | New file | None |
| `services-core/workspace.rs` (add `BasemindWorkspaceFs`) | Add to existing | Low (append only) |
| `tool-execution/document.rs` (add `basemind_extract_and_redact`) | Add to existing | Low |
| `file_read_tool.rs` (conversion call line) | **Line modified** | **High** — upstream refactors frequently |
| `LocalWorkspaceFs` injection point | **Line modified** | **High** |
| `lib.rs` (invoke_handler) | Line added to list | Low |
| `content.ts` (`FILE_VIEWER_TYPES`) | Element added to array | Low |
| `MarkdownRenderer.tsx` (sanitizeSchema, components) | Modify existing | Medium |
| `FilesPanel.tsx` (file click routing) | Conditional logic inserted | Medium-High |

### 15.2 Composition > Substitution

`BasemindWorkspaceFs` **decorates** `LocalWorkspaceFs` (not replaces) — upstream trait additions inherited automatically via delegation.

### 15.3 Isolated Crate

All Basemind logic in `basemind-integration/` — modified upstream files only import + branch, no business logic duplication.

### 15.4 Feature Flags

```rust
// file_read_tool.rs
#[cfg(feature = "basemind")]
let conversion_call = basemind_integration::basemind_extract_and_redact(bytes, resolved_path.to_string());
#[cfg(not(feature = "basemind"))]
let conversion_call = convert_document_to_markdown(bytes, resolved_path.to_string());
```

Upstream line preserved → clean 3-way merge on rebase.

### 15.5 Canary Tests

```rust
#[test]
fn upstream_conversion_signature_unchanged() {
    fn _assert_signature(_f: fn(Vec<u8>, String) -> futures::future::BoxFuture<'static, 
        Result<tool_runtime::fs::document::ConvertedDocument, tool_runtime::fs::document::DocumentConversionError>>) {}
}
```
Fails at **compile time** if upstream signature changes — better than silent prod regression.

### 15.6 Git Workflow

- Long-lived `basemind-integration` branch (never `main` directly)
- Atomic commits per interception point (one per §2, §3, §6, etc.)
- Regular rebase on upstream tagged releases (not continuous)
- `git rerere` enabled for automatic conflict resolution replay

---

## 16. Verification Checklist

| Layer | Command |
|-------|---------|
| Rust (basemind-integration) | `cargo test -p alex-basemind-integration` |
| Runtime-ports (RagSearchPort) | `cargo test -p alex-runtime-ports rag_search` |
| Services-core | `cargo test -p alex-services-core workspace` |
| Tool-execution | `cargo test -p tool-runtime document` |
| Assembly/core (tools) | `cargo test -p alex-core rag_search` |
| Boundary rule (no core → basemind dep) | `pnpm run check:core-boundaries` |
| Desktop (Tauri commands) | `cargo test -p alex-desktop basemind` |
| Desktop (pseudonymization) | `cargo test -p alex-desktop prompt_pseudonym` |
| Web UI (unit) | `pnpm --filter web-ui test BasemindAPI` |
| Web UI (E2E) | `pnpm --filter web-ui test:e2e redacted-document` |
| Web UI (E2E) | `pnpm --filter web-ui test:e2e prompt-pseudonymization` |
| Cross-platform | `pnpm run desktop:dev` (Linux/macOS/Windows) |
| Remote workspace | `pnpm run test:remote-workspace` (if exists) |

---

## 17. Dependencies on Basemind Fork (jamon8888/basemind)

**Pinned version:** `jamon8888/basemind` **v0.34.0** (2026-10-19), recorded in code as
`alex_basemind_integration::BASEMIND_VERSION` and checked against the running
daemon's published version before the first call. All PII/identity evidence in this spec
(issues 01–02, ADR 0001) was gathered at v0.33.0.

v0.34.0 is the upstream sync of `Goldziher/basemind` v0.29.3 into the fork (1,202
commits). The sync keeps the fork's PII, redaction and vault features end to end, so the
evidence in issues 01–02 still holds; the MCP surface it measured was re-verified against a
live v0.34.0 daemon, which serves `redact_text`, `memory`, `vault`, `code`, `git`,
`graph`, `shell`, `web`, `admin`, `agents` and `workspace`. Bundle v0.34.0: it ships the
`aarch64-unknown-linux-gnu` binary §8.1 requires. The upstream `Goldziher/basemind`
v0.29.x line is a **diverged numbering stream** (released after our tag, lower
numbers) and is not the integration target — do not reconcile its changelog entries
with ours.

**Upgrade note:** v0.34.0 advances the fork's release minor from 33 to 34, and the
persisted blob, index and git-cache schema versions derive from it. The first scan after
upgrading resets each view in place and rebuilds it once. The product's own state — the
identity registry and the session vault — carries its own schema and is unaffected.

**Transport:** the daemon serves MCP over streamable HTTP at `POST /mcp?root=<abs
workspace path>`, on the loopback address published in `<comms_dir>/http.addr` (line 1:
`host:port`, line 2: the bearer token the front-end requires). The client speaks
JSON-RPC 2.0 to that endpoint; there is no `/tools/call` route.

Assumes PR adds/provides:
- [ ] `extract_and_redact(bytes, path_hint) -> (markdown, entities[])`
- [ ] `reveal_entity(entity_id) -> original_value`
- [ ] `query_rag(query, top_k) -> [{chunk, source_doc, score, entity_ids}]`
- [ ] `detect_pii(text) -> findings[]` (for prompt pseudonymization)
- [ ] `vault_encrypt(map) -> encrypted_blob` / `vault_decrypt(blob) -> map` (session vault)
- [ ] Model download/verify CLI (`basemind model pull <name>`)
- [ ] Cross-platform binaries (Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64)
- [ ] Stable IPC protocol (stdio JSON-RPC or gRPC)
- [ ] OCR capability (for Option A in §5)

Where issue 01 measured reality, the MCP surface (`redact_text`,
`memory mode="documents"`, `vault`) plus the product-side identity layer (§4, §11)
supersede the assumed RPC shapes above (`query_rag`, `detect_pii`, `vault_encrypt`);
treat the list as the original expectation, not as an open dependency.

---

## 18. Gateway Wiring Pre-Staged for the Fork's v0.35 (ADR-0015, PR #93)

The fork's next release moves the privacy gateway into basemind (ADR-0015): a
workspace carrying a `safe/` mirror is **protected** — `redact_text` and `vault`
are withheld from the agent surface, and re-opened only for a basemind process
marked with `BASEMIND_GATEWAY_TOKEN`. The product **is** the gateway, so the
following is pre-staged now, against the v0.34.0 pin, and needs no further code
when v0.35 lands beyond the pin flip (§12 / `daemon::BASEMIND_VERSION` +
`resources/basemind/VERSION.json`):

- **Gateway capability** (`gateway.rs`): the token constant, a per-launch
  random token generator (OS randomness, never reused), and a fail-closed
  protected-workspace probe mirroring the fork's `safe`-entry check (a symlink
  counts; an inspection error counts as protected). The desktop launch path
  (`start_bundled_basemind_daemon`) sets the marker on the sidecar command's
  environment only — never process-wide — so an agent sandbox cannot inherit
  it. Residual risk stays where basemind's ADR-0015 records it.
- **NER coverage contract** (fork v0.34.1, additive): `ner_truncated` on a
  `redact_text` answer means PII past the cut was never redacted; per D7 the
  refusal stays loud. `ner_coverage_refusal` is enforced at all three
  consumers: the document redaction port, the RAG safety pass, and
  `RedactionStore::record` (nothing persists before the check). A v0.34.0
  sidecar that omits the field is unaffected.
- **Pseudonym caveat** (ADR-0015 §3): a dictionary value appears in the
  redacted text as the code name, while the rehydration map still carries the
  token. The store records the entity as revealable with zero occurrences —
  covered by `a_pseudonymised_value_stays_revealable_without_a_token_in_the_text`.
- **TAX_ID** (ADR-0015 §4): handled generically by `category_of` /
  `category_of_token`; the Web UI tone (`RevealableEntityBadge`) and the
  `entityTypes.TAX_ID` i18n keys already exist in all runtime locales.

Still pending the v0.35 landing:

- [ ] Flip the pin: `daemon::BASEMIND_VERSION`, `resources/basemind/VERSION.json`
      tag + checksums (`scripts/prepare-basemind-resource.mjs`).
- [ ] Decide whether `redact_text` calls should set `require_ner` (only safe
      once the NER model dir is confirmed present on first-use machines).
- [ ] Adopt the native `pseudonyms` field in place of the product's own
      prompt pseudonymization pass where the fork's dictionary is strictly
      better (§11.4); malformed entries abort the call fail-closed.
- [ ] A daemon started outside the product (`basemind comms start` by hand)
      carries no marker; our own `redact_text`/`vault` refs would then be
      refused in a protected workspace. If that surfaces in practice, surface
      a state, do not retry silently.

---

*End of Specification*