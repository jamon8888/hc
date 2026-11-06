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
  private opened = false;

  constructor(private readonly options: WordcraftEmbeddedDocumentSessionOptions) {}

  get state(): WordcraftDocumentHostState {
    return this.options.host.state;
  }

  handleBridgeEvent(event: WordcraftBridgeEvent): void {
    if (this.disposed) return;

    if (event.type === 'failed') {
      this.opened = false;
      this.options.onError?.(event.error);
    }

    this.options.host.handleEvent(event);
    this.options.onState?.(this.options.host.state);

    if (event.type !== 'ready') return;

    try {
      // A validated ready event after the iframe reloads is a recovery
      // handshake, not a duplicate. Re-open the document so the embedded
      // runtime cannot remain ready while showing an empty or stale surface.
      this.opened = false;
      this.options.bridge.openDocument(
        this.options.name,
        this.options.bytes,
        this.options.baseRevision ?? 0,
      );
      this.opened = true;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.options.onError?.(message);
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.options.host.dispose();
    this.options.bridge.dispose();
  }
}
