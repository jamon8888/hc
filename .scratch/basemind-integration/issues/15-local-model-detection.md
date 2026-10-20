# 15: Local model detection (TS + Rust)

**What to build:** Detection logic to identify local model endpoints (Ollama, LM Studio, vLLM) so pseudonymization can be skipped for them. Implemented in both TypeScript (Web UI) and Rust (Tauri backend).

**Blocked by:** 14 (settings schema for toggles)

**Status:** superseded (2026-10-16, decision D2 — re-scope record: `.scratch/pii-safe-rag-spine/issues/09-ticket-rescope-15-17-18.md`)

> **Superseded — do not build.** Local model detection exists only to *skip*
> pseudonymization for local endpoints. D2 removes the skip: pseudonymization runs for
> every model, cloud or local, so there is nothing to detect. `isLocalModelBaseUrl`,
> `shouldPseudonymize` and `is_local_model_base_url` are dead ends; any of them that
> already exists in code should be deleted or left unreferenced. The behavior that
> survives lives in ticket 17 (unconditional interception, the master toggle as the
> only skip condition) and ticket 18 (single `pseudonymize_prompts` switch).

- [ ] Create `src/web-ui/src/infrastructure/config/services/localModelDetection.ts`:
  ```typescript
  export function isLocalModelBaseUrl(baseUrl: string): boolean {
    const url = new URL(baseUrl);
    const hostname = url.hostname.toLowerCase();
    if (['localhost', '127.0.0.1', '::1'].includes(hostname)) return true;
    const ipv4 = hostname.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
    if (ipv4) {
      const [, a, b] = ipv4.map(Number);
      if (a === 10) return true;
      if (a === 172 && b >= 16 && b <= 31) return true;
      if (a === 192 && b === 168) return true;
      if (a === 169 && b === 254) return true;
    }
    if (hostname === 'host.docker.internal' || hostname.endsWith('.local')) return true;
    return false;
  }
  
  export async function shouldPseudonymize(sessionId: string): Promise<boolean> {
    const baseUrl = await getActiveModelBaseUrl(sessionId);
    if (!baseUrl) return true;
    return !isLocalModelBaseUrl(baseUrl);
  }
  ```
- [ ] Create `basemind-integration/src/local_model_detection.rs` with identical logic in Rust
- [ ] Export `is_local_model_base_url()` from `basemind-integration` crate