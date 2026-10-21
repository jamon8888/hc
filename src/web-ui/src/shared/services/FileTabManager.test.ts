import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  notifyError: vi.fn(),
  openContentInBestTarget: vi.fn(),
  captureContentScope: vi.fn(() => ({
    surfaceId: 'local',
    workspaceId: 'ws-1',
    workspacePath: '/tmp/workspace',
    remoteConnectionId: undefined as string | undefined,
  })),
  probeBasemindSurface: vi.fn(),
  isBasemindPeerRefused: vi.fn(() => false),
}));

vi.mock('@/infrastructure/api/service-api/ApiClient', () => ({
  api: { invoke: mocks.invoke },
}));

vi.mock('@/infrastructure/api/service-api/BasemindAPI', () => ({
  basemindAPI: { probeSurface: (...args: unknown[]) => mocks.probeBasemindSurface(...args) },
}));

vi.mock('@/infrastructure/basemind/basemindPeerPolicy', () => ({
  isBasemindPeerRefused: (...args: unknown[]) => mocks.isBasemindPeerRefused(...args),
}));

vi.mock('@/infrastructure/i18n', () => ({
  i18nService: Object.assign((key: string) => key, { t: (key: string) => key, getT: () => (key: string) => key }),
}));

vi.mock('@/shared/notification-system/services/NotificationService', () => ({
  notificationService: { error: mocks.notifyError },
}));

vi.mock('@/infrastructure/language-detection', () => ({
  getEditorType: (filePathOrName: string) =>
    filePathOrName.endsWith('.md') ? 'markdown-editor' : 'code-editor',
  isRedactedDocumentPath: (filePathOrName: string) => filePathOrName.endsWith('.pptx'),
}));

vi.mock('@/app/workbench/contentResourceStore', () => ({
  resourceFilePath: (filePath: string) => filePath,
}));

vi.mock('./workbenchContentService', async () => {
  const actualScope = {
    surfaceId: 'local',
    workspaceId: 'ws-1',
    workspacePath: '/tmp/workspace',
  };
  return {
    captureContentScope: () => actualScope,
    openContentInBestTarget: mocks.openContentInBestTarget,
  };
});

import { fileTabManager } from './FileTabManager';

const manager = fileTabManager;

describe('FileTabManager docx routing (tickets 13/17)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.invoke.mockReset();
  });

  it('routes an available capability to the WordCraft window instead of a tab', async () => {
    mocks.invoke.mockResolvedValue({
      status: 'available',
      cli: { command: 'wordcraft-cli', resolvedPath: '/c', source: 'managed' },
      app: { command: 'wordcraft', resolvedPath: '/app', source: 'managed' },
      version: '0.7.0',
      message: null,
      remediation: null,
      checkedAtEpochMs: 1,
    });
    manager.openFile({ filePath: '/tmp/workspace/report.docx' });

    // The open decision is async; flush it.
    await vi.waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith('wordcraft_open_document_command', {
        request: { path: '/tmp/workspace/report.docx', workspaceId: 'ws-1' },
      });
    });
    expect(mocks.notifyError).not.toHaveBeenCalled();
    expect(mocks.openContentInBestTarget).not.toHaveBeenCalled();
  });

  it('refuses a not-installed capability loudly without a code-editor dump', async () => {
    mocks.invoke.mockResolvedValue({
      status: 'not-installed',
      cli: null,
      app: null,
      version: null,
      message: 'Not downloaded yet',
      remediation: 'Download the pinned WordCraft build, then Recheck.',
      checkedAtEpochMs: 1,
    });
    manager.openFile({ filePath: '/tmp/workspace/report.docx' });

    await vi.waitFor(() => {
      expect(mocks.notifyError).toHaveBeenCalledWith('Download the pinned WordCraft build, then Recheck.', {
        title: 'errors:file.wordcraftOpenFailed',
      });
    });
    expect(mocks.openContentInBestTarget).not.toHaveBeenCalled();
  });

  it('launch failures surface the backend message verbatim, keeping the dump forbidden', async () => {
    const backendError = new Error('WordCraft document editing runs on the local workspace only.');
    mocks.invoke
      .mockResolvedValueOnce({
        status: 'available',
        app: { command: 'wordcraft', resolvedPath: '/app', source: 'managed' },
        cli: null,
        version: null,
        message: null,
        remediation: null,
        checkedAtEpochMs: 1,
      })
      .mockRejectedValueOnce(backendError);
    mocks.invoke
      .mockResolvedValueOnce({
        status: 'available',
        app: { command: 'wordcraft', resolvedPath: '/app', source: 'managed' },
        cli: null,
        version: null,
        message: null,
        remediation: null,
        checkedAtEpochMs: 1,
      })
      .mockRejectedValueOnce(backendError);

    manager.openFile({ filePath: '/tmp/workspace/report.docx', workspaceId: 'ws-remote' });

    await vi.waitFor(() => {
      expect(mocks.notifyError).toHaveBeenCalledWith(
        'WordCraft document editing runs on the local workspace only.',
        { title: 'errors:file.wordcraftOpenFailed' },
      );
    });
    expect(mocks.openContentInBestTarget).not.toHaveBeenCalled();
  });

  it('keeps the regular tab behavior where no capability probe exists', async () => {
    mocks.invoke.mockRejectedValue(new Error('wordcraft_status_command is unavailable here'));
    manager.openFile({ filePath: '/tmp/workspace/report.docx' });

    await vi.waitFor(() => {
      expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    });
    const content = mocks.openContentInBestTarget.mock.calls[0][0];
    expect(content.type).toBe('code-editor');
    expect(content.data.filePath).toBe('/tmp/workspace/report.docx');
    expect(mocks.notifyError).not.toHaveBeenCalled();
  });

  it('an explicit editor surface choice bypasses the WordCraft route entirely', async () => {
    manager.openFile({ filePath: '/tmp/workspace/report.docx', editorType: 'code-editor' });

    // Synchronous tab open, no capability probe at all.
    expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it('non-docx files keep the synchronous tab path without probing', async () => {
    manager.openFile({ filePath: '/tmp/workspace/notes.md' });

    expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});

describe('FileTabManager redacted document routing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.invoke.mockReset();
    mocks.probeBasemindSurface.mockReset();
    mocks.isBasemindPeerRefused.mockReset();
    mocks.isBasemindPeerRefused.mockReturnValue(false);
    mocks.probeBasemindSurface.mockResolvedValue(true);
  });

  it('opens a document format in the redacted viewer once the surface is available', async () => {
    manager.openFile({ filePath: '/tmp/workspace/deck.pptx' });

    await vi.waitFor(() => {
      expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    });
    const content = mocks.openContentInBestTarget.mock.calls[0][0];
    expect(content.type).toBe('redacted-document-viewer');
    expect(mocks.notifyError).not.toHaveBeenCalled();
  });

  it('keeps the plain editor and reports loudly when the surface is missing', async () => {
    mocks.probeBasemindSurface.mockResolvedValue(false);
    manager.openFile({ filePath: '/tmp/workspace/deck.pptx' });

    await vi.waitFor(() => {
      expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    });
    const content = mocks.openContentInBestTarget.mock.calls[0][0];
    expect(content.type).toBe('code-editor');
    expect(mocks.notifyError).toHaveBeenCalledWith('errors:file.redactedDocumentUnavailable', {
      title: 'errors:file.redactedDocumentUnavailable',
    });
  });

  it('opens a peer-owned document in the viewer, which states that redaction is unavailable', async () => {
    mocks.isBasemindPeerRefused.mockReturnValue(true);
    manager.openFile({ filePath: '/tmp/workspace/deck.pptx' });

    await vi.waitFor(() => {
      expect(mocks.openContentInBestTarget).toHaveBeenCalled();
    });
    const content = mocks.openContentInBestTarget.mock.calls[0][0];
    expect(content.type).toBe('redacted-document-viewer');
    expect(mocks.probeBasemindSurface).not.toHaveBeenCalled();
    expect(mocks.notifyError).not.toHaveBeenCalled();
  });
});
