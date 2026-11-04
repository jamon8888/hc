import { workspaceAPI } from './WorkspaceAPI';
import type {
  WordcraftBridgeEvent,
  WordcraftSaveMessage,
} from './WordcraftBridge';

export interface WordcraftDocumentHostState {
  dirty: boolean;
  revision: number;
  saving: boolean;
  error?: string;
}

export interface WordcraftDocumentHostOptions {
  sessionId: string;
  saveBytes: (bytes: Uint8Array, revision: number) => Promise<void>;
  acknowledgeSave: (requestId: string, revision: number, ok: boolean, error?: string) => void;
  onState?: (state: WordcraftDocumentHostState) => void;
}

interface PendingSave {
  event: WordcraftSaveMessage;
}

export class WordcraftDocumentHost {
  private disposed = false;
  private latestAcceptedRevision = 0;
  private saveInFlight = false;
  private readonly saveQueue: PendingSave[] = [];
  private currentState: WordcraftDocumentHostState = {
    dirty: false,
    revision: 0,
    saving: false,
  };

  constructor(private readonly options: WordcraftDocumentHostOptions) {}

  get state(): WordcraftDocumentHostState {
    return { ...this.currentState };
  }

  handleEvent(event: WordcraftBridgeEvent): void {
    if (this.disposed) return;
    if (event.type === 'dirty') {
      if (event.sessionId !== this.options.sessionId) return;
      if (event.revision < this.latestAcceptedRevision) return;
      this.latestAcceptedRevision = event.revision;
      if (!event.dirty && (this.saveInFlight || this.saveQueue.length > 0)) {
        this.update({ revision: event.revision, error: undefined });
        return;
      }
      this.update({ dirty: event.dirty, revision: event.revision, error: undefined });
      return;
    }
    if (event.type === 'failed') {
      if (event.sessionId && event.sessionId !== this.options.sessionId) return;
      this.update({ error: event.error, saving: false });
      return;
    }
    if (event.type === 'save') {
      if (event.sessionId !== this.options.sessionId) return;
      if (event.revision < this.latestAcceptedRevision) {
        this.options.acknowledgeSave(
          event.requestId,
          event.revision,
          false,
          'WordCraft save revision is stale',
        );
        return;
      }
      this.latestAcceptedRevision = event.revision;
      this.saveQueue.push({ event });
      this.update({ dirty: true, revision: event.revision, saving: true, error: undefined });
      void this.drainSaves();
    }
  }

  dispose(): void {
    this.disposed = true;
    this.saveQueue.length = 0;
    this.update({ saving: false });
  }

  private async drainSaves(): Promise<void> {
    if (this.disposed || this.saveInFlight || this.saveQueue.length === 0) return;

    const { event } = this.saveQueue.shift()!;
    this.saveInFlight = true;
    try {
      await this.options.saveBytes(toUint8Array(event.bytes), event.revision);
      if (this.disposed) return;
      this.update({
        ...(event.revision === this.latestAcceptedRevision && this.saveQueue.length === 0
          ? { dirty: false, revision: event.revision, error: undefined }
          : { dirty: true, revision: this.latestAcceptedRevision }),
        saving: this.saveQueue.length > 0,
      });
      this.options.acknowledgeSave(event.requestId, event.revision, true);
    } catch (error) {
      if (this.disposed) return;
      const message = error instanceof Error ? error.message : String(error);
      this.update({
        ...(event.revision === this.latestAcceptedRevision
          ? { dirty: true, revision: event.revision, error: message }
          : { dirty: true, revision: this.latestAcceptedRevision }),
        saving: this.saveQueue.length > 0,
      });
      this.options.acknowledgeSave(event.requestId, event.revision, false, message);
    } finally {
      this.saveInFlight = false;
      if (!this.disposed) void this.drainSaves();
    }
  }

  private update(patch: Partial<WordcraftDocumentHostState>): void {
    this.currentState = { ...this.currentState, ...patch };
    this.options.onState?.(this.state);
  }
}

export function createWordcraftWorkspaceSaver(
  workspaceId: string,
  filePath: string,
  getExpectedHash?: () => string | undefined,
): (bytes: Uint8Array, revision: number) => Promise<void> {
  return (bytes) =>
    workspaceAPI.writeWorkspaceFileBase64(
      workspaceId,
      filePath,
      bytesToBase64(bytes),
      getExpectedHash?.(),
    );
}

function toUint8Array(bytes: ArrayBuffer | Uint8Array): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

function bytesToBase64(bytes: Uint8Array): string {
  const chunks: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    chunks.push(String.fromCharCode(...bytes.subarray(offset, offset + 0x8000)));
  }
  return globalThis.btoa(chunks.join(''));
}
