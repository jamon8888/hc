import { describe, expect, it, vi } from 'vitest';
import { api } from './ApiClient';
import { WordcraftAPI } from './WordcraftAPI';

vi.mock('./ApiClient', () => ({
  api: {
    invoke: vi.fn(),
  },
}));

describe('WordcraftAPI', () => {
  it('creates an authenticated local web surface through the desktop adapter', async () => {
    vi.mocked(api.invoke).mockResolvedValueOnce({
      url: 'http://127.0.0.1:41000/index.html?wordcraftToken=token',
      origin: 'http://127.0.0.1:41000',
      sessionId: 'web-session-1',
      version: '0.7.0',
      protocolVersion: 1,
      commandCapabilities: ['get_text'],
    });

    await expect(new WordcraftAPI().hostWebSurface()).resolves.toMatchObject({
      sessionId: 'web-session-1',
      origin: 'http://127.0.0.1:41000',
    });
    expect(api.invoke).toHaveBeenCalledWith('wordcraft_web_host_command', { request: {} });
  });

  it('releases the web host session through the desktop adapter', async () => {
    vi.mocked(api.invoke).mockResolvedValueOnce(undefined);

    await new WordcraftAPI().releaseWebSurface('web-session-1');

    expect(api.invoke).toHaveBeenCalledWith('wordcraft_web_release_command', {
      request: { sessionId: 'web-session-1' },
    });
  });

  it('completes a correlated embedded command through the desktop adapter', async () => {
    vi.mocked(api.invoke).mockResolvedValueOnce(undefined);

    await new WordcraftAPI().completeEmbeddedCommand({
      requestId: 'request-1',
      ok: true,
      result: { revision: 3 },
    });

    expect(api.invoke).toHaveBeenCalledWith('wordcraft_complete_embedded_command', {
      request: {
        requestId: 'request-1',
        ok: true,
        result: { revision: 3 },
      },
    });
  });
});
