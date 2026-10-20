/**
 * Availability of the Basemind redaction surface.
 *
 * The `basemind_*` host commands exist only in a build that carries the
 * `basemind` capability, so availability is a host fact and not a Web UI
 * constant. The Web UI records the answer once and every consumer asks for it
 * instead of guessing, so a host without the capability degrades to the
 * previous behavior instead of opening an unusable viewer.
 */

/** Whether the active host implements the Basemind redaction commands. */
export type BasemindSurfaceState = 'unknown' | 'available' | 'unavailable';

/** Raised when the active host does not implement the Basemind redaction surface. */
export class BasemindUnavailableError extends Error {
  constructor(command: string) {
    super(`The active host does not implement the Basemind command ${command}`);
    this.name = 'BasemindUnavailableError';
  }
}

let surfaceState: BasemindSurfaceState = 'unknown';

export function getBasemindSurfaceState(): BasemindSurfaceState {
  return surfaceState;
}

export function isBasemindSurfaceAvailable(): boolean {
  return surfaceState === 'available';
}

export function markBasemindSurfaceAvailable(): void {
  surfaceState = 'available';
}

export function markBasemindSurfaceUnavailable(): void {
  surfaceState = 'unavailable';
}

/** Test-only: forget the probed host answer. */
export function resetBasemindSurfaceState(): void {
  surfaceState = 'unknown';
}

/**
 * Whether a failure means the host does not implement the command.
 *
 * The Tauri host answers an unregistered command with "Command <name> not
 * found" and the WebSocket host with "Unknown command: <name>". Both name the
 * command, so the check requires the command name plus an unknown/not-found
 * signal. An unrelated Basemind failure never hides the surface.
 */
export function isUnknownBasemindCommandError(error: unknown, command: string): boolean {
  const message = error instanceof Error ? error.message : String(error ?? '');
  const normalized = message.toLowerCase();
  if (!normalized.includes(command.toLowerCase())) return false;
  return normalized.includes('not found') || normalized.includes('unknown command');
}
