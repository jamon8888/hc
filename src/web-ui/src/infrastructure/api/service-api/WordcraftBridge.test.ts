import { describe, expect, it, vi } from 'vitest';
import { WordcraftBridge, type WordcraftCommandRequest } from './WordcraftBridge';
import { WORDCRAFT_WEB_REQUIRED_COMMANDS, WORDCRAFT_WEB_PROTOCOL_VERSION } from './WordcraftProtocol';

const source = {} as MessageEventSource;
const context = {
  expectedOrigin: 'https://alex.local',
  expectedSource: source,
  expectedSessionId: 'document-session-1',
  pinnedVersion: '0.7.0',
};

function readyEvent(): MessageEvent<unknown> {
  return {
    origin: context.expectedOrigin,
    source,
    data: {
      type: 'ready',
      sessionId: context.expectedSessionId,
      version: context.pinnedVersion,
      protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
      commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
    },
  } as MessageEvent<unknown>;
}

function resultEvent(request: WordcraftCommandRequest, overrides: Record<string, unknown> = {}) {
  return {
    origin: context.expectedOrigin,
    source,
    data: {
      type: 'result',
      requestId: request.requestId,
      sessionId: request.sessionId,
      revision: 2,
      ok: true,
      dirty: true,
      value: { changed: true },
      ...overrides,
    },
  } as MessageEvent<unknown>;
}

describe('WordcraftBridge', () => {
  it('does not send commands before the ready handshake', async () => {
    const postMessage = vi.fn();
    const bridge = new WordcraftBridge({ ...context, postMessage });

    await expect(bridge.request('list_commands', {}, 0, 'Alex Agent (session-1)'))
      .rejects.toThrow('WordCraft bridge is unavailable');
    expect(postMessage).not.toHaveBeenCalled();
  });

  it('ignores untrusted or pre-handshake events without poisoning the bridge', () => {
    const onEvent = vi.fn();
    const bridge = new WordcraftBridge({ ...context, postMessage: () => undefined, onEvent });

    expect(bridge.handleMessage({
      origin: 'https://untrusted.example',
      source,
      data: readyEvent().data,
    } as MessageEvent<unknown>)).toBe(false);
    expect(bridge.handleMessage({
      origin: context.expectedOrigin,
      source,
      data: { type: 'dirty', sessionId: context.expectedSessionId, revision: 1, dirty: true },
    } as MessageEvent<unknown>)).toBe(false);
    expect(onEvent).not.toHaveBeenCalled();
    expect(bridge.handleMessage(readyEvent())).toBe(true);
    expect(bridge.isReady).toBe(true);
  });

  it('correlates an allowlisted command with its result', async () => {
    const posted: Array<WordcraftCommandRequest | Record<string, unknown>> = [];
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: message => posted.push(message),
      requestIdFactory: () => 'request-1',
    });
    bridge.handleMessage(readyEvent());

    const pending = bridge.request('batch', { commands: [] }, 1, 'Alex Agent (session-1)');
    const request = posted[0] as WordcraftCommandRequest;
    expect(request).toMatchObject({
      type: 'command',
      requestId: 'request-1',
      sessionId: context.expectedSessionId,
      expectedRevision: 1,
      method: 'batch',
    });
    bridge.handleMessage(resultEvent(request));

    await expect(pending).resolves.toMatchObject({ requestId: 'request-1', revision: 2 });
  });

  it('rejects stale sessions and malformed results explicitly', async () => {
    const posted: WordcraftCommandRequest[] = [];
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: message => posted.push(message as WordcraftCommandRequest),
      timeoutMs: 20,
      requestIdFactory: () => 'request-2',
    });
    bridge.handleMessage(readyEvent());
    const pending = bridge.request('get_text', {}, 3, 'Alex Agent (session-1)');
    const request = posted[0];

    expect(bridge.handleMessage(resultEvent(request, { sessionId: 'other-session' }))).toBe(true);
    await expect(pending).rejects.toThrow('WordCraft result session identity is stale');

    const secondPending = bridge.request('get_text', {}, 3, 'Alex Agent (session-1)');
    const secondRequest = posted[1];
    expect(bridge.handleMessage(resultEvent(secondRequest, { revision: 'bad' }))).toBe(true);
    await expect(secondPending).rejects.toThrow('WordCraft result is malformed');
  });

  it('fails all pending commands when the frame reports a failure', async () => {
    const reject = vi.fn();
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: () => undefined,
      onEvent: event => {
        if (event.type === 'failed') reject(event.error);
      },
    });
    bridge.handleMessage(readyEvent());
    const pending = bridge.request('inspect_document', {}, 0, 'Alex Agent (session-1)');
    bridge.handleMessage({
      origin: context.expectedOrigin,
      source,
      data: { type: 'failed', sessionId: context.expectedSessionId, error: 'WebGPU unavailable' },
    } as MessageEvent<unknown>);

    await expect(pending).rejects.toThrow('WebGPU unavailable');
    expect(reject).toHaveBeenCalledWith('WebGPU unavailable');
    expect(bridge.isReady).toBe(false);
  });

  it('recovers after a failed frame sends a new valid ready message', () => {
    const bridge = new WordcraftBridge({ ...context, postMessage: () => undefined });
    bridge.handleMessage(readyEvent());
    bridge.handleMessage({
      origin: context.expectedOrigin,
      source,
      data: { type: 'failed', sessionId: context.expectedSessionId, error: 'frame reloaded' },
    } as MessageEvent<unknown>);
    expect(bridge.isReady).toBe(false);

    expect(bridge.handleMessage(readyEvent())).toBe(true);
    expect(bridge.isReady).toBe(true);
  });

  it('acknowledges durable save success or failure with the session identity', () => {
    const posted: Array<Record<string, unknown>> = [];
    const bridge = new WordcraftBridge({ ...context, postMessage: message => posted.push(message) });

    bridge.acknowledgeSave('save-1', 4, true);
    bridge.acknowledgeSave('save-2', 5, false, 'workspace write failed');

    expect(posted).toEqual([
      { type: 'save-ack', requestId: 'save-1', sessionId: context.expectedSessionId, revision: 4, ok: true },
      {
        type: 'save-ack',
        requestId: 'save-2',
        sessionId: context.expectedSessionId,
        revision: 5,
        ok: false,
        error: 'workspace write failed',
      },
    ]);
  });
});
