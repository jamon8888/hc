/**
 * Runtime validation for the WordCraft parent/iframe ready handshake.
 *
 * The native capability probe validates the pinned artifact metadata. This
 * seam validates the browser message boundary before Alex presents the frame:
 * browser origin, iframe source, session identity, message shape, protocol,
 * version, and command allowlist all have to agree.
 */

export const WORDCRAFT_WEB_PROTOCOL_VERSION = 1 as const;

export const WORDCRAFT_WEB_REQUIRED_COMMANDS = [
  'list_commands',
  'execute',
  'batch',
  'get_text',
  'inspect_document',
  'render_page',
  'save_document',
] as const;

export interface WordcraftReadyMessage {
  type: 'ready';
  sessionId: string;
  version: string;
  protocolVersion: number;
  commandCapabilities: string[];
}

export interface WordcraftReadyValidationContext {
  expectedOrigin: string;
  expectedSource: MessageEventSource;
  expectedSessionId: string;
  pinnedVersion: string;
}

export type WordcraftReadyValidation =
  | { ok: true; ready: WordcraftReadyMessage }
  | { ok: false; reason: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

/** Validate an iframe ready event; every failure is a non-ready result. */
export function validateWordcraftReadyEvent(
  event: MessageEvent<unknown>,
  context: WordcraftReadyValidationContext
): WordcraftReadyValidation {
  if (event.origin !== context.expectedOrigin) {
    return { ok: false, reason: 'WordCraft ready message origin is not trusted' };
  }
  if (event.source !== context.expectedSource) {
    return { ok: false, reason: 'WordCraft ready message source is not trusted' };
  }
  if (!isRecord(event.data) || event.data.type !== 'ready') {
    return { ok: false, reason: 'WordCraft ready message is malformed' };
  }

  const { sessionId, version, protocolVersion, commandCapabilities } = event.data;
  if (
    typeof sessionId !== 'string' ||
    typeof version !== 'string' ||
    typeof protocolVersion !== 'number' ||
    !Number.isInteger(protocolVersion) ||
    !isStringArray(commandCapabilities)
  ) {
    return { ok: false, reason: 'WordCraft ready message is malformed' };
  }
  if (sessionId !== context.expectedSessionId) {
    return { ok: false, reason: 'WordCraft ready message session identity is stale' };
  }
  if (protocolVersion !== WORDCRAFT_WEB_PROTOCOL_VERSION) {
    return { ok: false, reason: 'WordCraft ready message protocol is unsupported' };
  }
  if (version !== context.pinnedVersion) {
    return { ok: false, reason: 'WordCraft ready message version is not pinned' };
  }

  const missing = WORDCRAFT_WEB_REQUIRED_COMMANDS.filter(
    (command) => !commandCapabilities.includes(command)
  );
  if (missing.length > 0) {
    return {
      ok: false,
      reason: `WordCraft ready message is missing command capabilities: ${missing.join(', ')}`,
    };
  }

  return {
    ok: true,
    ready: {
      type: 'ready',
      sessionId,
      version,
      protocolVersion,
      commandCapabilities,
    },
  };
}
