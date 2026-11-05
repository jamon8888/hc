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

  it('serializes commands and posts the next request after the prior result', async () => {
    const posted: Array<WordcraftCommandRequest | Record<string, unknown>> = [];
    let nextRequestId = 0;
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: message => posted.push(message),
      requestIdFactory: () => `ordered-${++nextRequestId}`,
    });
    bridge.handleMessage(readyEvent());

    const firstPending = bridge.request('execute', { command: 'text.insert' }, 1, 'Alex Agent (session-1)');
    const secondPending = bridge.request('get_text', {}, 2, 'Alex Agent (session-1)');

    expect(posted).toHaveLength(1);
    const firstRequest = posted[0] as WordcraftCommandRequest;
    bridge.handleMessage(resultEvent(firstRequest, { revision: 2 }));

    await vi.waitFor(() => expect(posted).toHaveLength(2));
    const secondRequest = posted[1] as WordcraftCommandRequest;
    expect(secondRequest).toMatchObject({ requestId: 'ordered-2', method: 'get_text' });
    bridge.handleMessage(resultEvent(secondRequest, { revision: 3 }));

    await expect(firstPending).resolves.toMatchObject({ requestId: 'ordered-1', revision: 2 });
    await expect(secondPending).resolves.toMatchObject({ requestId: 'ordered-2', revision: 3 });
  });

  it('rejects stale sessions and malformed results explicitly', async () => {
    const posted: WordcraftCommandRequest[] = [];
    const requestIds = ['request-2', 'request-3'];
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: message => posted.push(message as WordcraftCommandRequest),
      timeoutMs: 20,
      requestIdFactory: () => requestIds.shift()!,
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

  it('rejects a result older than the requested document revision', async () => {
    const posted: WordcraftCommandRequest[] = [];
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: message => posted.push(message as WordcraftCommandRequest),
    });
    bridge.handleMessage(readyEvent());

    const pending = bridge.request('get_text', {}, 8, 'Alex Agent (session-1)');
    const request = posted[0];
    expect(bridge.handleMessage(resultEvent(request, { revision: 7 }))).toBe(true);

    await expect(pending).rejects.toThrow('WordCraft result revision is stale');
  });

  it('does not reuse a request ID after a timeout', async () => {
    let nextId = 0;
    const bridge = new WordcraftBridge({
      ...context,
      postMessage: () => undefined,
      timeoutMs: 5,
      requestIdFactory: () => `reused-${nextId++ % 1}`,
    });
    bridge.handleMessage(readyEvent());

    await expect(bridge.request('get_text', {}, 0, 'Alex Agent (session-1)')).rejects.toThrow('timed out');
    await expect(bridge.request('get_text', {}, 0, 'Alex Agent (session-1)')).rejects.toThrow('already used');
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

  it('fails queued commands when the frame reports a failure', async () => {
    const bridge = new WordcraftBridge({ ...context, postMessage: vi.fn() });
    bridge.handleMessage(readyEvent());
    const firstPending = bridge.request('get_text', {}, 0, 'Alex Agent (session-1)');
    const secondPending = bridge.request('inspect_document', {}, 0, 'Alex Agent (session-1)');

    bridge.handleMessage({
      origin: context.expectedOrigin,
      source,
      data: { type: 'failed', sessionId: context.expectedSessionId, error: 'frame closed' },
    } as MessageEvent<unknown>);

    await expect(firstPending).rejects.toThrow('frame closed');
    await expect(secondPending).rejects.toThrow('frame closed');
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
