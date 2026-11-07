import type {
  WordcraftBridge,
  WordcraftBridgeEvent,
} from './WordcraftBridge';
import type {
  WordcraftDocumentHost,
  WordcraftDocumentHostState,
} from './WordcraftDocumentHost';

export interface WordcraftEmbeddedDocumentSessionOptions {
  bridge: WordcraftBridge;
  host: WordcraftDocumentHost;
  name: string;
  bytes: Uint8Array;
  baseRevision?: number;
  onState?: (state: WordcraftDocumentHostState) => void;
  onError?: (error: string) => void;
}

/**
 * Coordinates the parent-side document lifecycle without owning a browser
 * frame. The presenter supplies bridge messages and forwards bridge events;
 * this seam owns the ordering between ready, open, dirty, save, and teardown.
 */
export class WordcraftEmbeddedDocumentSession {
  private disposed = false;
  private openedGeneration = 0;

  constructor(private readonly options: WordcraftEmbeddedDocumentSessionOptions) {}

  get state(): WordcraftDocumentHostState {
    return this.options.host.state;
  }

  handleBridgeEvent(event: WordcraftBridgeEvent): void {
    if (this.disposed) return;

    if (event.type === 'failed') {
      this.openedGeneration = 0;
      this.options.onError?.(event.error);
    }

    this.options.host.handleEvent(event);
    this.options.onState?.(this.options.host.state);

    if (event.type !== 'ready') return;
    const generation = event.generation ?? 1;
    if (generation <= this.openedGeneration) return;

    try {
      // A validated ready event after the iframe reloads is a recovery
      // handshake, not a duplicate. Re-open the document so the embedded
      // runtime cannot remain ready while showing an empty or stale surface.
      this.options.bridge.openDocument(
        this.options.name,
        this.options.host.getLatestBytes(this.options.bytes),
        this.options.host.state.revision || this.options.baseRevision || 0,
      );
      this.openedGeneration = generation;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.options.onError?.(message);
    }
  }

  async dispose(timeoutMs?: number): Promise<void> {
    if (this.disposed) {
      await this.options.host.waitForDispose(timeoutMs);
      return;
    }
    this.disposed = true;
    this.options.bridge.dispose();
    await this.options.host.waitForDispose(timeoutMs);
  }
}
