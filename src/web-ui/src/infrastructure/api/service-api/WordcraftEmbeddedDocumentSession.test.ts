import { describe, expect, it, vi } from 'vitest';
import { WordcraftBridge, type WordcraftBridgeOutboundMessage } from './WordcraftBridge';
import { WordcraftDocumentHost } from './WordcraftDocumentHost';
import { WordcraftEmbeddedDocumentSession } from './WordcraftEmbeddedDocumentSession';
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

describe('WordcraftEmbeddedDocumentSession', () => {
  it('opens the document after ready and forwards host state', () => {
    const posted: WordcraftBridgeOutboundMessage[] = [];
    const bridge = new WordcraftBridge({ ...context, postMessage: message => posted.push(message) });
    const host = new WordcraftDocumentHost({
      sessionId: context.expectedSessionId,
      saveBytes: vi.fn(),
      acknowledgeSave: vi.fn(),
    });
    const onState = vi.fn();
    const session = new WordcraftEmbeddedDocumentSession({
      bridge,
      host,
      name: 'draft.docx',
      bytes: new Uint8Array([1, 2, 3]),
      baseRevision: 4,
      onState,
    });

    bridge.handleMessage(readyEvent());
    expect(() => session.handleBridgeEvent({
      type: 'ready',
      ready: {
        type: 'ready',
        sessionId: context.expectedSessionId,
        version: context.pinnedVersion,
        protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
        commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
      },
    })).not.toThrow();
    expect(posted).toMatchObject([{
      type: 'open',
      sessionId: context.expectedSessionId,
      name: 'draft.docx',
      baseRevision: 4,
    }]);

    session.handleBridgeEvent({
      type: 'dirty',
      sessionId: context.expectedSessionId,
      revision: 5,
      dirty: true,
    });
    expect(session.state).toMatchObject({ dirty: true, revision: 5 });
    expect(onState).toHaveBeenCalled();
  });

  it('reopens after a frame failure and a new ready event', () => {
    const posted: WordcraftBridgeOutboundMessage[] = [];
    const bridge = new WordcraftBridge({ ...context, postMessage: message => posted.push(message) });
    const host = new WordcraftDocumentHost({
      sessionId: context.expectedSessionId,
      saveBytes: vi.fn(),
      acknowledgeSave: vi.fn(),
    });
    const onError = vi.fn();
    const session = new WordcraftEmbeddedDocumentSession({
      bridge,
      host,
      name: 'draft.docx',
      bytes: new Uint8Array([1]),
      onError,
    });

    bridge.handleMessage(readyEvent());
    session.handleBridgeEvent({
      type: 'ready',
      ready: {
        type: 'ready',
        sessionId: context.expectedSessionId,
        version: context.pinnedVersion,
        protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
        commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
      },
    });
    session.handleBridgeEvent({
      type: 'failed',
      sessionId: context.expectedSessionId,
      error: 'frame reloaded',
    });
    bridge.handleMessage(readyEvent());
    session.handleBridgeEvent({
      type: 'ready',
      ready: {
        type: 'ready',
        sessionId: context.expectedSessionId,
        version: context.pinnedVersion,
        protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
        commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
      },
    });

    expect(posted.filter(message => message.type === 'open')).toHaveLength(2);
    expect(onError).toHaveBeenCalledWith('frame reloaded');
  });

  it('reopens when the iframe reloads and handshakes without a failure event', () => {
    const posted: WordcraftBridgeOutboundMessage[] = [];
    const bridge = new WordcraftBridge({ ...context, postMessage: message => posted.push(message) });
    const host = new WordcraftDocumentHost({
      sessionId: context.expectedSessionId,
      saveBytes: vi.fn(),
      acknowledgeSave: vi.fn(),
    });
    const session = new WordcraftEmbeddedDocumentSession({
      bridge,
      host,
      name: 'draft.docx',
      bytes: new Uint8Array([1]),
    });
    const ready = (generation: number) => ({
      type: 'ready' as const,
      generation,
      ready: {
        type: 'ready' as const,
        sessionId: context.expectedSessionId,
        version: context.pinnedVersion,
        protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
        commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
      },
    });

    bridge.handleMessage(readyEvent());
    session.handleBridgeEvent(ready(1));
    bridge.handleMessage(readyEvent());
    session.handleBridgeEvent(ready(2));

    expect(posted.filter(message => message.type === 'open')).toHaveLength(2);
  });

  it('disposes both bridge and host and ignores later events', () => {
    const bridge = new WordcraftBridge({ ...context, postMessage: vi.fn() });
    const host = new WordcraftDocumentHost({
      sessionId: context.expectedSessionId,
      saveBytes: vi.fn(),
      acknowledgeSave: vi.fn(),
    });
    const onState = vi.fn();
    const session = new WordcraftEmbeddedDocumentSession({
      bridge,
      host,
      name: 'draft.docx',
      bytes: new Uint8Array([1]),
      onState,
    });

    session.dispose();
    session.handleBridgeEvent({
      type: 'dirty',
      sessionId: context.expectedSessionId,
      revision: 1,
      dirty: true,
    });

    expect(session.state).toEqual({ dirty: false, revision: 0, saving: false });
    expect(onState).not.toHaveBeenCalled();
    expect(bridge.isReady).toBe(false);
  });
});
