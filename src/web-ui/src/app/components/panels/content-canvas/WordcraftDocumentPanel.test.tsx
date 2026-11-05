// @vitest-environment jsdom

import React, { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  hostWebSurface: vi.fn(),
  statusQuiet: vi.fn(),
  openDocument: vi.fn(),
  readWorkspaceFile: vi.fn(),
  t: vi.fn((key: string) => key),
}));

vi.mock('@/infrastructure/api/service-api/WordcraftAPI', () => ({
  wordcraftAPI: {
    hostWebSurface: mocks.hostWebSurface,
    statusQuiet: mocks.statusQuiet,
    openDocument: mocks.openDocument,
  },
}));

vi.mock('@/infrastructure/api/service-api/WorkspaceAPI', () => ({
  workspaceAPI: {
    readWorkspaceFile: mocks.readWorkspaceFile,
    writeWorkspaceFileBase64: vi.fn(),
  },
}));

vi.mock('@/infrastructure/api/service-api/WordcraftBridge', () => ({ WordcraftBridge: class {} }));
vi.mock('@/infrastructure/api/service-api/WordcraftDocumentHost', () => ({ WordcraftDocumentHost: class {} }));
vi.mock('@/infrastructure/api/service-api/WordcraftEmbeddedDocumentSession', () => ({
  WordcraftEmbeddedDocumentSession: class {},
}));
vi.mock('@/infrastructure/i18n', () => ({ useI18n: () => ({ t: mocks.t }) }));
vi.mock('@/infrastructure/peer-device/deviceSurface', () => ({ isLocalSurface: () => true }));
vi.mock('@/tools/editor/services/EditorDocument', () => ({ useEditorDocument: () => null }));
vi.mock('@/tools/editor/services/DocumentAgentContext', () => ({
  registerDocumentAgentContext: vi.fn(),
  setDocumentAgentContextActive: vi.fn(),
  unregisterDocumentAgentContext: vi.fn(),
  updateDocumentAgentContext: vi.fn(),
}));
vi.mock('@/shared/utils/logger', () => ({
  createLogger: () => ({ error: vi.fn(), warn: vi.fn(), info: vi.fn(), debug: vi.fn() }),
}));

import WordcraftDocumentPanel from './WordcraftDocumentPanel';

describe('WordcraftDocumentPanel fallback routing', () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    mocks.hostWebSurface.mockReset().mockRejectedValue(new Error('embedded protocol unavailable'));
    mocks.statusQuiet.mockReset().mockResolvedValue({
      status: 'available',
      app: { command: 'wordcraft', resolvedPath: '/app', source: 'managed' },
      remediation: null,
      message: null,
    });
    mocks.openDocument.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function render(surfaceId?: string) {
    act(() => {
      root.render(
        <WordcraftDocumentPanel
          filePath="/workspace/report.docx"
          fileName="report.docx"
          workspaceId="workspace-1"
          surfaceId={surfaceId}
        />,
      );
    });
  }

  it('offers native WordCraft when the embedded host cannot start', async () => {
    render('local');
    await act(async () => {});

    const fallback = await vi.waitFor(() =>
      container.querySelector<HTMLButtonElement>('[data-alex-component="wordcraft-fallback"]'),
    );
    expect(fallback?.textContent).toContain('flexiblePanel.wordcraft.openNative');

    await act(async () => {
      fallback?.click();
    });
    expect(mocks.openDocument).toHaveBeenCalledWith({
      path: '/workspace/report.docx',
      workspaceId: 'workspace-1',
    });
    expect(container.textContent).toContain('flexiblePanel.wordcraft.nativeOpened');
  });

  it('refuses to default a missing surface identity to the local desktop', async () => {
    render();
    await act(async () => {});

    await vi.waitFor(() => {
      expect(container.textContent).toContain('flexiblePanel.wordcraft.surfaceRequired');
    });
    expect(mocks.hostWebSurface).not.toHaveBeenCalled();
    expect(mocks.readWorkspaceFile).not.toHaveBeenCalled();
  });

  it('shows remediation without opening a binary or code fallback', async () => {
    mocks.statusQuiet.mockResolvedValue({
      status: 'not-installed',
      app: null,
      remediation: 'Install WordCraft to open .docx documents.',
      message: 'WordCraft is not installed.',
    });
    render('local');
    await act(async () => {});

    await vi.waitFor(() => {
      expect(container.textContent).toContain('Install WordCraft to open .docx documents.');
    });
    expect(container.querySelector('[data-alex-component="wordcraft-fallback"]')).toBeNull();
    expect(mocks.openDocument).not.toHaveBeenCalled();
  });
});
