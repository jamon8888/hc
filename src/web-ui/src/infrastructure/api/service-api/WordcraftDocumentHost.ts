import { workspaceAPI } from './WorkspaceAPI';
import type {
  WordcraftBridgeEvent,
  WordcraftSaveMessage,
} from './WordcraftBridge';
import { createLogger } from '@/shared/utils/logger';

const log = createLogger('WordcraftDocumentHost');

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

export class WordcraftDocumentHost {
  private disposed = false;
  private nextEventToken = 0;
  private latestEventToken = 0;
  private readonly pendingSaves = new Map<number, number>();
  private saveQueue: Promise<void> = Promise.resolve();
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
      this.latestEventToken = ++this.nextEventToken;
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
      const token = ++this.nextEventToken;
      this.latestEventToken = token;
      this.pendingSaves.set(token, event.revision);
      this.update({ dirty: true, revision: event.revision, saving: true, error: undefined });
      this.saveQueue = this.saveQueue
        .then(() => this.persist(event, token))
        .catch((error) => {
          log.warn('WordCraft save queue recovered after an unexpected persistence failure', { error });
        });
    }
  }

  dispose(): void {
    this.disposed = true;
    this.update({ saving: false });
  }

  private async persist(event: WordcraftSaveMessage, token: number): Promise<void> {
    let ok = false;
    let errorMessage: string | undefined;
    try {
      await this.options.saveBytes(toUint8Array(event.bytes), event.revision);
      if (this.disposed) return;
      this.pendingSaves.delete(token);
      const isLatest = this.latestEventToken === token;
      this.update({
        ...(isLatest ? { dirty: false, revision: event.revision, error: undefined } : {}),
        saving: this.pendingSaves.size > 0,
      });
      ok = true;
    } catch (error) {
      if (this.disposed) return;
      this.pendingSaves.delete(token);
      const message = error instanceof Error ? error.message : String(error);
      errorMessage = message;
      this.update({
        ...(this.latestEventToken === token ? { dirty: true, revision: event.revision, error: message } : {}),
        saving: this.pendingSaves.size > 0,
      });
    }

    try {
      if (errorMessage === undefined) {
        this.options.acknowledgeSave(event.requestId, event.revision, ok);
      } else {
        this.options.acknowledgeSave(event.requestId, event.revision, ok, errorMessage);
      }
    } catch (error) {
      log.warn('Failed to acknowledge WordCraft save', {
        requestId: event.requestId,
        revision: event.revision,
        error,
      });
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
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  if (view.byteLength === 0) {
    throw new Error('WordCraft save payload must be a non-empty binary buffer');
  }
  return view;
}

function bytesToBase64(bytes: Uint8Array): string {
  const chunks: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    chunks.push(String.fromCharCode(...bytes.subarray(offset, offset + 0x8000)));
  }
  return globalThis.btoa(chunks.join(''));
}
