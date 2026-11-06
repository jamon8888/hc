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
  durableSaveTimeoutMs?: number;
}

interface PendingSave {
  event: WordcraftSaveMessage;
}

interface DurableSaveWaiter {
  revision: number;
  resolve: () => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class WordcraftDocumentHost {
  private disposed = false;
  private disposeRequested = false;
  private latestAcceptedRevision = 0;
  private saveInFlight = false;
  private readonly saveQueue: PendingSave[] = [];
  private readonly durableSaveWaiters = new Set<DurableSaveWaiter>();
  private durableRevision = -1;
  private latestBytes: Uint8Array;
  private currentState: WordcraftDocumentHostState = {
    dirty: false,
    revision: 0,
    saving: false,
  };

  constructor(private readonly options: WordcraftDocumentHostOptions, initialBytes?: Uint8Array) {
    this.latestBytes = initialBytes ? new Uint8Array(initialBytes) : new Uint8Array();
  }

  get state(): WordcraftDocumentHostState {
    return { ...this.currentState };
  }

  getLatestBytes(fallback?: Uint8Array): Uint8Array {
    const bytes = this.latestBytes.length > 0 ? this.latestBytes : fallback ?? this.latestBytes;
    return new Uint8Array(bytes);
  }

  waitForDurableSave(revision: number): Promise<void> {
    if (!Number.isSafeInteger(revision) || revision < 0) {
      return Promise.reject(new Error('WordCraft save revision is invalid'));
    }
    if (this.durableRevision >= revision) return Promise.resolve();
    if (this.disposed) return Promise.reject(new Error('WordCraft document host is closed'));

    return new Promise((resolve, reject) => {
      const timeoutMs = this.options.durableSaveTimeoutMs ?? 10_000;
      const waiter: DurableSaveWaiter = {
        revision,
        resolve,
        reject,
        timer: setTimeout(() => {
          this.durableSaveWaiters.delete(waiter);
          reject(new Error(`WordCraft durable save timed out at revision ${revision}`));
        }, timeoutMs),
      };
      this.durableSaveWaiters.add(waiter);
    });
  }

  handleEvent(event: WordcraftBridgeEvent): void {
    if (this.disposed || this.disposeRequested) return;
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
        this.acknowledgeSave(event, false, 'WordCraft save revision is stale');
        return;
      }
      this.latestAcceptedRevision = event.revision;
      this.saveQueue.push({ event });
      this.update({ dirty: true, revision: event.revision, saving: true, error: undefined });
      void this.drainSaves();
    }
  }

  dispose(): void {
    if (this.disposed || this.disposeRequested) return;
    this.disposeRequested = true;
    if (!this.saveInFlight && this.saveQueue.length === 0) {
      this.finishDispose();
    }
  }

  private async drainSaves(): Promise<void> {
    if (this.disposed || this.saveInFlight || this.saveQueue.length === 0) return;

    const { event } = this.saveQueue.shift()!;
    this.saveInFlight = true;
    let ok = false;
    let errorMessage: string | undefined;
    try {
      try {
        const bytes = toUint8Array(event.bytes);
        this.latestBytes = bytes.slice();
        await this.options.saveBytes(bytes, event.revision);
        if (this.disposed) return;
        this.durableRevision = Math.max(this.durableRevision, event.revision);
        this.resolveDurableSaveWaiters(event.revision);
        this.update({
          ...(event.revision === this.latestAcceptedRevision && this.saveQueue.length === 0
            ? { dirty: false, revision: event.revision, error: undefined }
            : { dirty: true, revision: this.latestAcceptedRevision }),
          saving: this.saveQueue.length > 0,
        });
        ok = true;
      } catch (error) {
        if (this.disposed) return;
        const message = error instanceof Error ? error.message : String(error);
        errorMessage = message;
        this.update({
          ...(event.revision === this.latestAcceptedRevision
            ? { dirty: true, revision: event.revision, error: message }
            : { dirty: true, revision: this.latestAcceptedRevision }),
          saving: this.saveQueue.length > 0,
        });
        this.rejectDurableSaveWaiters(event.revision, message);
      }
      if (!this.disposed) this.acknowledgeSave(event, ok, errorMessage);
    } finally {
      this.saveInFlight = false;
      if (this.disposeRequested && this.saveQueue.length === 0) {
        this.finishDispose();
      } else if (!this.disposed) {
        void this.drainSaves();
      }
    }
  }

  private finishDispose(): void {
    this.disposed = true;
    this.saveQueue.length = 0;
    this.update({ saving: false });
    for (const waiter of this.durableSaveWaiters) {
      clearTimeout(waiter.timer);
      waiter.reject(new Error('WordCraft document host is closed'));
    }
    this.durableSaveWaiters.clear();
  }

  private acknowledgeSave(event: WordcraftSaveMessage, ok: boolean, error?: string): void {
    try {
      if (error === undefined) {
        this.options.acknowledgeSave(event.requestId, event.revision, ok);
      } else {
        this.options.acknowledgeSave(event.requestId, event.revision, ok, error);
      }
    } catch (acknowledgementError) {
      log.warn('Failed to acknowledge WordCraft save', {
        requestId: event.requestId,
        revision: event.revision,
        error: acknowledgementError,
      });
    }
  }

  private update(patch: Partial<WordcraftDocumentHostState>): void {
    this.currentState = { ...this.currentState, ...patch };
    this.options.onState?.(this.state);
  }

  private resolveDurableSaveWaiters(revision: number): void {
    for (const waiter of this.durableSaveWaiters) {
      if (waiter.revision > revision) continue;
      clearTimeout(waiter.timer);
      this.durableSaveWaiters.delete(waiter);
      waiter.resolve();
    }
  }

  private rejectDurableSaveWaiters(revision: number, message: string): void {
    for (const waiter of this.durableSaveWaiters) {
      if (waiter.revision !== revision) continue;
      clearTimeout(waiter.timer);
      this.durableSaveWaiters.delete(waiter);
      waiter.reject(new Error(message));
    }
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
