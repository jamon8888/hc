# Spec — OpenBitFun + Basemind Integration (Cross-Platform Ready)

**Status:** Draft Technical Specification  
**Scope:** Complete integration of Basemind (jamon8888/basemind fork, PR in review) as the document extraction/redaction/RAG engine across all OpenBitFun surfaces (Desktop, CLI, Web UI, Mobile Web, Remote Connect). Models download locally on first use and are configurable in Settings.

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

**No constraint on LLM used (cloud or local)** — guarantees rest on what *reaches* the LLM.

---

## 2. Interception #1 — Workspace Filesystem Access

**File:** `src/crates/services/services-core/src/workspace.rs`  
**Target:** `LocalWorkspaceFs` (implements `WorkspaceFileSystem` from `openbitfun_runtime_ports`)

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

**File to create:** `src/crates/assembly/core/src/agentic/tools/implementations/rag_search_tool.rs`

```rust
pub struct RagSearchTool { basemind: Arc<BasemindClient> }

#[async_trait]
impl Tool for RagSearchTool {
    fn name(&self) -> &str { "rag_search" }
    
    // Input: { query: string, top_k?: number }
    // Output: [{ chunk: string, source_doc: string, score: number, entity_ids: string[] }]
    async fn execute(&self, input: Value, context: &ToolUseContext) -> ToolResult {
        let query = input["query"].as_str().ok_or("query required")?;
        let top_k = input["top_k"].as_u64().unwrap_or(8) as usize;
        let hits = self.basemind.query_rag(query, top_k).await?;
        // hits.chunk is already redacted (indexed from redacted markdown)
        ToolResult::json(hits)
    }
}
```

**Registration:** Add to tool registry in `assembly/core/src/agentic/tools/registry.rs` (feature-gated).

**Feature flag:** `tools-rag` in `assembly/core/Cargo.toml`.

**Content indexed:** Exclusively from `basemind_extract_and_redact` output → automatically inherits I1.

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
- Tooltip from `@openbitfun/ui` on hover
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
│ [Source]   [Redacted (12 PII)]               │  ← Toolbar/ToolbarGroup (@openbitfun/ui)
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

**Tauri sidecar config** (`src-tauri/tauri.conf.json`):
```json
{
  "bundle": {
    "sidecar": [
      { "path": "../bin/basemind-linux-x86_64", "arch": "x86_64" },
      { "path": "../bin/basemind-linux-aarch64", "arch": "aarch64" },
      { "path": "../bin/basemind-macos-x86_64", "arch": "x86_64" },
      { "path": "../bin/basemind-macos-arm64", "arch": "aarch64" },
      { "path": "../bin/basemind-windows-x86_64.exe", "arch": "x86_64" }
    ]
  }
}
```

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

**Persistence:** Model metadata in `openbitfun_core::service::config::GlobalConfig` → `basemind.models` section.

---

## 9. Settings Integration

### 9.1 Config Schema Extension

```rust
// openbitfun_core::service::config::types
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

**Registry entry** (Product Operation Registry): `src/crates/contracts/product-domains/src/remote_surface/table.rs` — add row for `basemind_reveal_entity` and `basemind_read_redacted_document` with `RemoteWorkspaceStance::Supported`.

---

## 11. User Prompt Pseudonymization (Local-Model Aware)

### 11.1 Objective

Pseudonymize user prompts in the composer before sending to cloud LLMs, while **skipping pseudonymization for local models** (Ollama, LM Studio, vLLM, etc.) since data never leaves the machine. RAG search remains available for both.

**Invariant I9:** No PII from user prompts reaches cloud LLMs. Local models receive raw prompts.

### 11.2 Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ USER PROMPT PSEUDONYMIZATION FLOW                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Composer (ChatInput.tsx)                                                   │
│       │                                                                     │
│       ▼                                                                     │
│  useMessageSender.ts ── sends: message (raw), displayMessage (raw)         │
│       │                                                                     │
│       ▼                                                                     │
│  LocalSessionDriver.startTurn()                                            │
│       │                                                                     │
│       ▼                                                                     │
│  agentAPI.startDialogTurn({ user_input, original_user_input })             │
│       │                                                                     │
│       ▼                    ┌──────────────────────────────────────────┐    │
│  Tauri: start_dialog_turn │  PSEUDONYMIZATION INTERCEPTION POINT     │    │
│       │                   │  desktop_dialog_turn_request()           │    │
│       ▼                   │  1. Get active model's base_url          │    │
│  desktop_dialog_turn_     │  2. If local (localhost/private IP) →    │    │
│  request()                │     SKIP pseudonymization                │    │
│       │                   │  3. If cloud → run PII detection         │    │
│       ▼                   │  4. Replace with tokens ([PERSON_1])     │    │
│  AgentRuntime.submit_    │  5. Store rehydration map in session     │    │
│  dialog_turn()            │     vault (encrypted)                    │    │
│       │                   │  6. Send pseudonymized → agent           │    │
│       ▼                   │  7. Keep original_user_input for display │    │
│  Agent Loop               │                                           │    │
│  (LLM sees pseudonymized  └──────────────────────────────────────────┘    │
│   text for cloud;       ─── HISTORY DISPLAY ───                          │
│   raw for local)        Session restore → rehydrate from vault →         │
│                         show original values                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 11.3 Local Model Detection

**New file:** `src/web-ui/src/infrastructure/config/services/localModelDetection.ts`

```typescript
/** Check if a base URL points to a local/private endpoint */
export function isLocalModelBaseUrl(baseUrl: string): boolean {
  try {
    const url = new URL(baseUrl);
    const hostname = url.hostname.toLowerCase();
    
    // Explicit localhost
    if (hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '::1') {
      return true;
    }
    
    // Private IP ranges (RFC 1918)
    const ipv4Match = hostname.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
    if (ipv4Match) {
      const [_, a, b] = ipv4Match.map(Number);
      // 10.0.0.0/8
      if (a === 10) return true;
      // 172.16.0.0/12
      if (a === 172 && b >= 16 && b <= 31) return true;
      // 192.168.0.0/16
      if (a === 192 && b === 168) return true;
      // 169.254.0.0/16 (link-local)
      if (a === 169 && b === 254) return true;
    }
    
    // Docker/container hostnames
    if (hostname === 'host.docker.internal' || hostname.endsWith('.local')) {
      return true;
    }
    
    return false;
  } catch {
    return false;
  }
}

/** Determine if pseudonymization should run for a session */
export async function shouldPseudonymize(sessionId: string): Promise<boolean> {
  const baseUrl = await getActiveModelBaseUrl(sessionId);
  if (!baseUrl) return true; // Default to pseudonymize if unknown
  
  return !isLocalModelBaseUrl(baseUrl);
}
```

**Rust equivalent** in `basemind-integration/src/lib.rs`:

```rust
pub fn is_local_model_base_url(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else { return false; };
    let Some(hostname) = url.host_str() else { return false; };
    let hostname = hostname.to_lowercase();
    
    // Explicit localhost
    if matches!(hostname.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return true;
    }
    
    // Private IPv4 ranges
    if let Ok(ip) = hostname.parse::<std::net::Ipv4Addr>() {
        let octets = ip.octets();
        if octets[0] == 10 { return true; }                    // 10.0.0.0/8
        if octets[0] == 172 && (16..=31).contains(&octets[1]) { return true; } // 172.16.0.0/12
        if octets[0] == 192 && octets[1] == 168 { return true; } // 192.168.0.0/16
        if octets[0] == 169 && octets[1] == 254 { return true; } // 169.254.0.0/16
    }
    
    // Docker/container hostnames
    if hostname == "host.docker.internal" || hostname.ends_with(".local") {
        return true;
    }
    
    false
}
```

### 11.4 Interception Point

**File:** `src/apps/desktop/src/api/agentic_api.rs` — `desktop_dialog_turn_request()`

```rust
fn desktop_dialog_turn_request(
    request: StartDialogTurnRequest,
    state: &AppState,  // Need AppState for config access
) -> Result<AgentDialogTurnRequest, String> 
{
    // ... existing code ...
    
    // NEW: Check if pseudonymization should run
    #[cfg(feature = "basemind")]
    let should_pseudonymize = {
        // Get the model config for this session
        let model_name = request.agent_type.clone(); // or from session config
        let base_url = get_model_base_url(&state, &model_name).await?;
        !basemind_integration::is_local_model_base_url(&base_url)
    };
    
    #[cfg(not(feature = "basemind"))]
    let should_pseudonymize = false;

    // NEW: Pseudonymize only for cloud models
    #[cfg(feature = "basemind")]
    let (pseudonymized_input, rehydration_map) = if should_pseudonymize {
        let pseudonymizer = basemind_integration::get_prompt_pseudonymizer(&state)?;
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                pseudonymizer.pseudonymize(&request.user_input, &request.session_id).await
            })
        })?
    } else {
        (request.user_input.clone(), HashMap::new())
    };
    
    #[cfg(not(feature = "basemind"))]
    let (pseudonymized_input, rehydration_map) = (request.user_input.clone(), HashMap::new());

    // Store rehydration map in request metadata for debugging
    let mut metadata = desktop_user_message_metadata(request.user_message_metadata);
    if !rehydration_map.is_empty() {
        metadata.insert(
            "prompt_rehydration".to_string(),
            serde_json::to_value(rehydration_map).unwrap_or_default()
        );
    }

    Ok(AgentDialogTurnRequest {
        session_id: request.session_id,
        message: pseudonymized_input,           // ← LLM sees this (pseudonymized for cloud, raw for local)
        original_message: request.original_user_input.or(Some(request.user_input)), // ← UI sees this
        // ... rest unchanged
        metadata,
    })
}
```

### 11.5 Prompt Pseudonymizer Implementation

**New file:** `basemind-integration/src/prompt_pseudonym.rs`

```rust
use basemind::pii::{translate_findings, FindingInput, ChunkSpan};
use basemind::vault::{vault_encrypt, vault_decrypt};

pub struct PromptPseudonymizer {
    basemind_client: Arc<BasemindClient>,
    vault: Arc<SessionVault>,
}

impl PromptPseudonymizer {
    /// Pseudonymize user prompt, return (pseudonymized_text, rehydration_map)
    pub async fn pseudonymize(&self, text: &str, session_id: &str) 
        -> Result<(String, HashMap<String, String>), PseudonymError> 
    {
        // 1. Run PII detection via basemind (reuse pii::pipeline)
        let findings = self.basemind_client.detect_pii(text).await?;
        
        // 2. Build rehydration map: token → original value
        let mut rehydration_map = HashMap::new();
        let mut pseudonymized = text.to_string();
        
        // Sort by byte offset descending so replacements don't shift indices
        let mut sorted_findings = findings;
        sorted_findings.sort_by(|a, b| b.start.cmp(&a.start));
        
        for (idx, finding) in sorted_findings.iter().enumerate() {
            let token = format!("[{}_{}]", finding.category.to_uppercase(), idx + 1);
            let original = &text[finding.start as usize..finding.end as usize];
            rehydration_map.insert(token.clone(), original.to_string());
            
            // Replace in pseudonymized text
            pseudonymized.replace_range(
                finding.start as usize..finding.end as usize, 
                &token
            );
        }
        
        // 3. Encrypt and store in session vault
        let vault_key = format!("session:{}:prompt_rehydration", session_id);
        self.vault.encrypt_and_store(&vault_key, &rehydration_map).await?;
        
        Ok((pseudonymized, rehydration_map))
    }
    
    /// Rehydrate a pseudonymized text for display
    pub async fn rehydrate(&self, text: &str, session_id: &str) -> Result<String, PseudonymError> {
        let vault_key = format!("session:{}:prompt_rehydration", session_id);
        let map = self.vault.decrypt_and_load(&vault_key).await?;
        
        let mut result = text.to_string();
        // Sort by token length descending to avoid partial replacements
        let mut tokens: Vec<_> = map.keys().collect();
        tokens.sort_by(|a, b| b.len().cmp(&a.len()));
        
        for token in tokens {
            if let Some(original) = map.get(token) {
                result = result.replace(token, original);
            }
        }
        Ok(result)
    }
}
```

### 11.6 Session Vault

**New file:** `basemind-integration/src/session_vault.rs`

```rust
use basemind::vault::{vault_encrypt, vault_decrypt};

pub struct SessionVault {
    vault_path: PathBuf,
}

impl SessionVault {
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path }
    }
    
    pub async fn encrypt_and_store(&self, key: &str, map: &HashMap<String, String>) -> Result<(), VaultError> {
        let encrypted = vault_encrypt(map)?;
        let path = self.vault_path.join(format!("{}.vault", key.replace(':', "_")));
        tokio::fs::write(path, encrypted).await?;
        Ok(())
    }
    
    pub async fn decrypt_and_load(&self, key: &str) -> Result<HashMap<String, String>, VaultError> {
        let path = self.vault_path.join(format!("{}.vault", key.replace(':', "_")));
        let encrypted = tokio::fs::read_to_string(path).await?;
        vault_decrypt(&encrypted)
    }
    
    pub async fn delete(&self, key: &str) -> Result<(), VaultError> {
        let path = self.vault_path.join(format!("{}.vault", key.replace(':', "_")));
        let _ = tokio::fs::remove_file(path).await;
        Ok(())
    }
}
```

### 11.7 History Rehydration

When loading session history (`RestoreSessionWithTurnsResponse`), rehydrate user messages:

```rust
// In agentic_api.rs restore session functions
async fn restore_session_with_turns(...) {
    // ... existing code ...
    
    #[cfg(feature = "basemind")]
    {
        let pseudonymizer = basemind_integration::get_prompt_pseudonymizer(&state)?;
        for turn in turns.iter_mut() {
            if let Some(user_msg) = turn.user_message.as_mut() {
                // Rehydrate the display content
                user_msg.content = pseudonymizer.rehydrate(&user_msg.content, &session_id).await
                    .unwrap_or_else(|_| user_msg.content.clone());
            }
        }
    }
    
    Ok(RestoreSessionWithTurnsResponse { session, turns })
}
```

### 11.8 Settings UI

**File:** `src/web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx` — add to Basemind settings:

```tsx
interface BasemindSettings {
  enabled: boolean;
  pseudonymize_prompts: boolean;           // Master toggle
  pseudonymize_local_models: boolean;      // Override for local models (default: false)
  auto_download_models: boolean;
  // ...
}

// In the Basemind settings panel:
<SettingRow>
  <SettingLabel>Pseudonymize user prompts</SettingLabel>
  <SettingDescription>
    Replace PII in your prompts before sending to cloud models. 
    Local models (Ollama, LM Studio, etc.) are excluded by default.
  </SettingDescription>
  <Switch 
    checked={settings.pseudonymize_prompts} 
    onChange={v => updateSettings({ pseudonymize_prompts: v })} 
  />
</SettingRow>

<SettingRow>
  <SettingLabel>Also pseudonymize for local models</SettingLabel>
  <SettingDescription>
    When enabled, pseudonymization runs even for local models (Ollama, LM Studio). 
    Normally not needed since data stays on your machine.
  </SettingDescription>
  <Switch 
    checked={settings.pseudonymize_local_models} 
    onChange={v => updateSettings({ pseudonymize_local_models: v })} 
    disabled={!settings.pseudonymize_prompts}
  />
</SettingRow>
```

### 11.9 Decision Matrix

| Scenario | Model Type | Pseudonymization | RAG |
|----------|------------|------------------|-----|
| User types "Jean Dupont lives at 123 Main St" | **Cloud** (OpenAI, Anthropic, etc.) | ✅ **Yes** — `[PERSON_1]` `[ADDRESS_1]` sent to LLM | ✅ Available |
| User types same prompt | **Local** (Ollama, LM Studio, vLLM) | ❌ **No** — raw text sent to local LLM | ✅ Available |
| User enables "Also pseudonymize local" | **Local** | ✅ Yes (user choice) | ✅ Available |
| Document read via `Read` tool | Any | ✅ Always (via BasemindWorkspaceFs) | ✅ Available |

### 11.10 Performance Impact

| Model Type | Pseudonymization | Added Latency |
|------------|------------------|---------------|
| **Cloud** | ✅ Runs (10-30ms PII detection + 1-5ms vault) | ~15-40ms |
| **Local** | ❌ Skipped | **0ms** |
| **Local (forced)** | ✅ Runs | ~15-40ms |

### 11.11 Remote/Peer/Detached Scenarios

| Scenario | Behavior |
|----------|----------|
| **Remote Workspace** | Pseudonymization runs on **controller** (where user types). Vault stored locally on controller. |
| **Peer Device Mode** | Controller pseudonymizes; peer never sees raw prompt. |
| **Detached Dispatch** | Controller pseudonymizes before dispatch; target only sees pseudonymized. |

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
| `src/crates/services/basemind-integration/` | **New crate** (Cargo.toml, lib.rs, workspace_fs.rs, document_convert.rs, models.rs, ipc.rs, prompt_pseudonym.rs, session_vault.rs, local_model_detection.rs) | §2, §3, §8, §11 |
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
| `web-ui/src/infrastructure/config/services/localModelDetection.ts` | **New** — local model detection logic | §11.3 |
| `web-ui/src/flow_chat/hooks/useMessageSender.ts` | Pass `session_id` context for detection | §11.4 |
| `src-tauri/tauri.conf.json` | Add Basemind sidecar binaries per platform | §8.1 |
| `Cargo.toml` (workspace) | Add `basemind-integration` to workspace members | — |

---

## 14. Open Questions / Decisions Needed

- [ ] **Basemind OCR ready?** Determines §5 Option A vs B.
- [ ] **Redact filenames in `read_dir`?** (§2.1) — currently only content redacted.
- [ ] **Exact Basemind IPC protocol** — stdio vs Unix socket vs gRPC loopback? Affects `ipc.rs` implementation.
- [ ] **Cache strategy** — reuse `DocumentCache` or dedicated Basemind cache (redaction latency higher)?
- [ ] **Model download source** — GitHub Releases vs Hugging Face Hub vs self-hosted?
- [ ] **Remote Basemind binary sync** — how to ensure remote host has matching version?
- [ ] **PII detection scope for prompts** — only user text, or also attached context (file paths, @mentions)?
- [ ] **Token format consistency** — use same `[PERSON_1]` format as documents, or distinct `{{PERSON_1}}`?
- [ ] **Vault storage location** — per-session file in session storage, or global vault with session keys?

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
| Rust (basemind-integration) | `cargo test -p basemind-integration` |
| Services-core | `cargo test -p openbitfun_services_core workspace` |
| Tool-execution | `cargo test -p openbitfun_tool_execution document` |
| Assembly/core (tools) | `cargo test -p openbitfun_assembly_core rag_search` |
| Desktop (Tauri commands) | `cargo test -p openbitfun_desktop basemind` |
| Desktop (pseudonymization) | `cargo test -p openbitfun_desktop prompt_pseudonym` |
| Web UI (unit) | `pnpm --filter web-ui test BasemindAPI` |
| Web UI (unit) | `pnpm --filter web-ui test localModelDetection` |
| Web UI (E2E) | `pnpm --filter web-ui test:e2e redacted-document` |
| Web UI (E2E) | `pnpm --filter web-ui test:e2e prompt-pseudonymization` |
| Cross-platform | `pnpm run desktop:dev` (Linux/macOS/Windows) |
| Remote workspace | `pnpm run test:remote-workspace` (if exists) |

---

## 17. Dependencies on Basemind Fork (jamon8888/basemind)

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

---

*End of Specification*