import {
  WORDCRAFT_WEB_REQUIRED_COMMANDS,
  validateWordcraftReadyEvent,
  type WordcraftReadyMessage,
  type WordcraftReadyValidationContext,
} from './WordcraftProtocol';

export const WORDCRAFT_BRIDGE_COMMANDS = WORDCRAFT_WEB_REQUIRED_COMMANDS;

export type WordcraftBridgeCommand = (typeof WORDCRAFT_BRIDGE_COMMANDS)[number];

export interface WordcraftCommandRequest {
  type: 'command';
  requestId: string;
  sessionId: string;
  expectedRevision: number;
  author: string;
  method: WordcraftBridgeCommand;
  params: Record<string, unknown>;
}

export interface WordcraftCommandResult {
  type: 'result';
  requestId: string;
  sessionId: string;
  revision: number;
  ok: boolean;
  value?: unknown;
  error?: string;
  dirty: boolean;
}

export interface WordcraftSaveMessage {
  type: 'save';
  requestId: string;
  sessionId: string;
  revision: number;
  name: string;
  bytes: ArrayBuffer | Uint8Array;
}

export interface WordcraftSaveAck {
  type: 'save-ack';
  requestId: string;
  sessionId: string;
  revision: number;
  ok: boolean;
  error?: string;
}

export interface WordcraftBridgeReadyEvent {
  type: 'ready';
  ready: WordcraftReadyMessage;
}

export interface WordcraftBridgeDirtyEvent {
  type: 'dirty';
  sessionId: string;
  revision: number;
  dirty: boolean;
}

export interface WordcraftBridgeFailedEvent {
  type: 'failed';
  sessionId?: string;
  error: string;
}

export type WordcraftBridgeEvent =
  | WordcraftBridgeReadyEvent
  | WordcraftBridgeDirtyEvent
  | WordcraftSaveMessage
  | WordcraftBridgeFailedEvent;

export interface WordcraftBridgeOptions extends WordcraftReadyValidationContext {
  timeoutMs?: number;
  postMessage: (message: WordcraftCommandRequest | WordcraftSaveAck) => void;
  onEvent?: (event: WordcraftBridgeEvent) => void;
  requestIdFactory?: () => string;
}

interface PendingCommand {
  method: WordcraftBridgeCommand;
  timer: ReturnType<typeof setTimeout>;
  resolve: (result: WordcraftCommandResult) => void;
  reject: (error: Error) => void;
}

export class WordcraftBridge {
  private readonly pending = new Map<string, PendingCommand>();
  private readonly issuedRequestIds = new Set<string>();
  private readonly timeoutMs: number;
  private readonly requestIdFactory: () => string;
  private disposed = false;
  private failed = false;
  private ready = false;
  private counter = 0;

  constructor(private readonly options: WordcraftBridgeOptions) {
    this.timeoutMs = options.timeoutMs ?? 10_000;
    this.requestIdFactory = options.requestIdFactory ?? (() => {
      this.counter += 1;
      return `wordcraft-request-${this.counter}`;
    });
  }

  get isReady(): boolean {
    return !this.disposed && !this.failed && this.ready;
  }

  handleMessage(event: MessageEvent<unknown>): boolean {
    if (this.disposed) return false;

    const data = event.data;
    if (isRecord(data) && data.type === 'ready') {
      if (!this.isTrustedEvent(event)) return false;
      const validation = validateWordcraftReadyEvent(event, this.options);
      if (!validation.ok) {
        this.fail(validation.reason);
        return false;
      }
      this.failed = false;
      this.ready = true;
      this.options.onEvent?.({ type: 'ready', ready: validation.ready });
      return true;
    }

    if (!this.isReady || !this.isTrustedEvent(event) || !isRecord(data)) return false;
    if (data.type === 'failed') {
      if (typeof data.error !== 'string') {
        this.fail('WordCraft failed message is malformed');
        return false;
      }
      if (data.sessionId !== undefined && data.sessionId !== this.options.expectedSessionId) {
        return false;
      }
      this.fail(data.error);
      return true;
    }
    if (data.type === 'dirty') {
      if (!isSessionRevision(data, this.options.expectedSessionId) || typeof data.dirty !== 'boolean') {
        return false;
      }
      this.options.onEvent?.({
        type: 'dirty',
        sessionId: data.sessionId,
        revision: data.revision,
        dirty: data.dirty,
      });
      return true;
    }
    if (data.type === 'save') {
      if (!isSessionRevision(data, this.options.expectedSessionId) ||
          typeof data.requestId !== 'string' || typeof data.name !== 'string' ||
          !isBytes(data.bytes)) {
        return false;
      }
      this.options.onEvent?.({
        type: 'save',
        requestId: data.requestId,
        sessionId: data.sessionId,
        revision: data.revision,
        name: data.name,
        bytes: data.bytes,
      });
      return true;
    }
    if (data.type === 'result') {
      return this.handleResult(data);
    }
    return false;
  }

  request(
    method: WordcraftBridgeCommand,
    params: Record<string, unknown>,
    expectedRevision: number,
    author: string,
  ): Promise<WordcraftCommandResult> {
    if (!this.isReady) return Promise.reject(new Error('WordCraft bridge is unavailable'));
    if (!isWordcraftBridgeCommand(method)) {
      return Promise.reject(new Error(`WordCraft command '${String(method)}' is not supported`));
    }
    if (!isRecord(params)) {
      return Promise.reject(new Error(`WordCraft command '${method}' parameters are malformed`));
    }
    const requestId = this.requestIdFactory();
    if (!requestId || this.pending.has(requestId) || this.issuedRequestIds.has(requestId)) {
      return Promise.reject(new Error('WordCraft bridge request ID is invalid or already used'));
    }
    this.issuedRequestIds.add(requestId);
    const request: WordcraftCommandRequest = {
      type: 'command',
      requestId,
      sessionId: this.options.expectedSessionId,
      expectedRevision,
      author,
      method,
      params,
    };
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        reject(new Error(`WordCraft command '${method}' timed out`));
      }, this.timeoutMs);
      this.pending.set(requestId, { method, timer, resolve, reject });
      try {
        this.options.postMessage(request);
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(requestId);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  acknowledgeSave(
    requestId: string,
    revision: number,
    ok: boolean,
    error?: string,
  ): void {
    if (this.disposed || !requestId || !Number.isSafeInteger(revision) || revision < 0) return;
    const ack: WordcraftSaveAck = {
      type: 'save-ack',
      requestId,
      sessionId: this.options.expectedSessionId,
      revision,
      ok,
      ...(error ? { error } : {}),
    };
    this.options.postMessage(ack);
  }

  dispose(reason = 'WordCraft bridge closed'): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(new Error(reason));
    }
    this.pending.clear();
  }

  private handleResult(data: Record<string, unknown>): boolean {
    if (typeof data.requestId !== 'string') {
      return false;
    }
    const pending = this.pending.get(data.requestId);
    if (!pending) return false;
    if (data.sessionId !== this.options.expectedSessionId) {
      this.rejectPending(data.requestId, 'WordCraft result session identity is stale');
      return true;
    }
    if (!isSessionRevision(data, this.options.expectedSessionId) ||
        typeof data.ok !== 'boolean' || typeof data.dirty !== 'boolean') {
      this.rejectPending(data.requestId, 'WordCraft result is malformed');
      return true;
    }
    this.pending.delete(data.requestId);
    clearTimeout(pending.timer);
    const result: WordcraftCommandResult = {
      type: 'result',
      requestId: data.requestId,
      sessionId: data.sessionId,
      revision: data.revision,
      ok: data.ok,
      dirty: data.dirty,
      ...(data.value !== undefined ? { value: data.value } : {}),
      ...(typeof data.error === 'string' ? { error: data.error } : {}),
    };
    if (!result.ok && !result.error) {
      pending.reject(new Error(`WordCraft command '${pending.method}' failed without an error`));
    } else {
      pending.resolve(result);
    }
    return true;
  }

  private rejectPending(requestId: string, reason: string): void {
    const pending = this.pending.get(requestId);
    if (!pending) return;
    this.pending.delete(requestId);
    clearTimeout(pending.timer);
    pending.reject(new Error(reason));
  }

  private isTrustedEvent(event: MessageEvent<unknown>): boolean {
    return event.origin === this.options.expectedOrigin && event.source === this.options.expectedSource;
  }

  private fail(reason: string): void {
    this.ready = false;
    this.failed = true;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(new Error(reason));
    }
    this.pending.clear();
    this.options.onEvent?.({ type: 'failed', sessionId: this.options.expectedSessionId, error: reason });
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isSessionRevision(
  value: Record<string, unknown>,
  expectedSessionId: string,
): value is Record<string, unknown> & { sessionId: string; revision: number } {
  return value.sessionId === expectedSessionId &&
    typeof value.sessionId === 'string' &&
    typeof value.revision === 'number' &&
    Number.isSafeInteger(value.revision) &&
    value.revision >= 0;
}

function isBytes(value: unknown): value is ArrayBuffer | Uint8Array {
  return value instanceof ArrayBuffer || value instanceof Uint8Array;
}

function isWordcraftBridgeCommand(value: unknown): value is WordcraftBridgeCommand {
  return typeof value === 'string' &&
    (WORDCRAFT_BRIDGE_COMMANDS as readonly string[]).includes(value);
}
