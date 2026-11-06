import { describe, expect, it } from 'vitest';
import {
  validateWordcraftReadyEvent,
  WORDCRAFT_WEB_REQUIRED_COMMANDS,
  WORDCRAFT_WEB_PROTOCOL_VERSION,
} from './WordcraftProtocol';

const source = {} as MessageEventSource;
const context = {
  expectedOrigin: 'https://alex.local',
  expectedSource: source,
  expectedSessionId: 'document-session-1',
  pinnedVersion: '0.7.0',
};

function readyEvent(
  dataOverrides: Record<string, unknown> = {},
  eventOverrides: Partial<MessageEvent<unknown>> = {}
): MessageEvent<unknown> {
  return {
    origin: context.expectedOrigin,
    source,
    data: {
      type: 'ready',
      sessionId: context.expectedSessionId,
      version: context.pinnedVersion,
      protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION,
      commandCapabilities: [...WORDCRAFT_WEB_REQUIRED_COMMANDS],
      ...dataOverrides,
    },
    ...eventOverrides,
  } as MessageEvent<unknown>;
}

describe('validateWordcraftReadyEvent', () => {
  it('accepts a matching ready message', () => {
    const result = validateWordcraftReadyEvent(readyEvent(), context);

    expect(result).toEqual({
      ok: true,
      ready: expect.objectContaining({
        type: 'ready',
        sessionId: context.expectedSessionId,
      }),
    });
  });

  it('fails closed for an untrusted origin', () => {
    const result = validateWordcraftReadyEvent(
      readyEvent({}, { origin: 'https://untrusted.example' }),
      context
    );

    expect(result).toEqual({
      ok: false,
      reason: 'WordCraft ready message origin is not trusted',
    });
  });

  it('fails closed for an unexpected source', () => {
    const result = validateWordcraftReadyEvent(
      readyEvent({}, { source: {} as MessageEventSource }),
      context
    );

    expect(result).toEqual({
      ok: false,
      reason: 'WordCraft ready message source is not trusted',
    });
  });

  it.each([
    ['stale session', { sessionId: 'other-session' }],
    ['unpinned version', { version: '0.8.0' }],
    ['unsupported protocol', { protocolVersion: WORDCRAFT_WEB_PROTOCOL_VERSION + 1 }],
    ['malformed payload', { type: 'ready', commandCapabilities: 'not-an-array' }],
  ])('fails closed for a %s message', (_label, dataOverrides) => {
    const result = validateWordcraftReadyEvent(readyEvent(dataOverrides), context);

    expect(result.ok).toBe(false);
  });

  it('fails closed when a required command is missing', () => {
    const [missingCommand, ...presentCommands] = WORDCRAFT_WEB_REQUIRED_COMMANDS;
    const result = validateWordcraftReadyEvent(
      readyEvent({ commandCapabilities: presentCommands }),
      context,
    );

    expect(result).toEqual({
      ok: false,
      reason: `WordCraft ready message is missing command capabilities: ${missingCommand}`,
    });
  });
});
