import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  BasemindUnavailableError,
  getBasemindSurfaceState,
  isUnknownBasemindCommandError,
  resetBasemindSurfaceState,
} from '@/infrastructure/basemind/basemindSurface';
import { basemindAPI } from './BasemindAPI';

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock('./ApiClient', () => ({
  api: {
    invoke: invokeMock,
  },
}));

describe('BasemindAPI', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    resetBasemindSurfaceState();
  });

  it('recognizes an unimplemented command from both host transports', () => {
    expect(isUnknownBasemindCommandError(
      new Error('Command basemind_reveal_entity not found'),
      'basemind_reveal_entity',
    )).toBe(true);
    expect(isUnknownBasemindCommandError(
      new Error('Unknown command: basemind_read_redacted_document'),
      'basemind_read_redacted_document',
    )).toBe(true);
  });

  it('does not hide the surface behind an unrelated failure', () => {
    expect(isUnknownBasemindCommandError(
      new Error('File is not valid UTF-8: /srv/deck.pptx'),
      'basemind_read_redacted_document',
    )).toBe(false);
    expect(isUnknownBasemindCommandError(
      new Error('Command basemind_reveal_entity failed'),
      'basemind_reveal_entity',
    )).toBe(false);
  });

  it('reports the surface as unavailable when the host does not implement it', async () => {
    invokeMock.mockRejectedValue(new Error('Command basemind_reveal_entity not found'));

    await expect(basemindAPI.probeSurface()).resolves.toBe(false);
    expect(getBasemindSurfaceState()).toBe('unavailable');
  });

  it('reports the surface as available when the host implements the command', async () => {
    invokeMock.mockRejectedValue(new Error('entity not found'));

    await expect(basemindAPI.probeSurface()).resolves.toBe(true);
    expect(getBasemindSurfaceState()).toBe('available');
  });

  it('probes the host at most once', async () => {
    invokeMock.mockRejectedValue(new Error('entity not found'));

    await basemindAPI.probeSurface();
    await basemindAPI.probeSurface();

    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('raises an unavailable error instead of a raw host failure', async () => {
    invokeMock.mockRejectedValue(new Error('Command basemind_reveal_entity not found'));

    await expect(basemindAPI.revealEntity('p1', 'workspace-1')).rejects.toBeInstanceOf(BasemindUnavailableError);
    expect(getBasemindSurfaceState()).toBe('unavailable');
  });

  it('passes the owning workspace to the reveal command', async () => {
    invokeMock.mockResolvedValue('Ada Lovelace');

    await expect(basemindAPI.revealEntity('p1', 'workspace-1')).resolves.toBe('Ada Lovelace');
    expect(invokeMock).toHaveBeenCalledWith('basemind_reveal_entity', {
      request: { workspaceId: 'workspace-1', entityId: 'p1' },
    });
  });
});
