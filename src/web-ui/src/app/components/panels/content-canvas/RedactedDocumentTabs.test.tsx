// @vitest-environment jsdom

import React, { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { RedactedDocumentTabs } from './RedactedDocumentTabs';

const mocks = vi.hoisted(() => ({
  readRedactedDocument: vi.fn(),
  revealEntity: vi.fn(),
  readWorkspaceFile: vi.fn(),
  isBasemindPeerRefused: vi.fn(),
  markdownProps: [] as Array<Record<string, unknown>>,
}));

vi.mock('@/infrastructure/api', () => ({
  basemindAPI: {
    readRedactedDocument: (...args: unknown[]) => mocks.readRedactedDocument(...args),
    revealEntity: (...args: unknown[]) => mocks.revealEntity(...args),
  },
  workspaceAPI: {
    readWorkspaceFile: (...args: unknown[]) => mocks.readWorkspaceFile(...args),
  },
}));

vi.mock('@/infrastructure/markdown/MarkdownRenderer', () => ({
  MarkdownRenderer: (props: Record<string, unknown>) => {
    mocks.markdownProps.push(props);
    return <div data-testid="markdown-renderer">{String(props.content ?? '')}</div>;
  },
}));

vi.mock('@/infrastructure/basemind/basemindPeerPolicy', () => ({
  isBasemindPeerRefused: (...args: unknown[]) => mocks.isBasemindPeerRefused(...args),
}));

vi.mock('@/infrastructure/i18n', () => ({
  useI18n: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      options && Object.keys(options).length > 0
        ? `${key}:${JSON.stringify(options)}`
        : key,
  }),
}));

vi.mock('@/shared/utils/logger', () => ({
  createLogger: () => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  }),
}));

describe('RedactedDocumentTabs', () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    mocks.readRedactedDocument.mockReset();
    mocks.readWorkspaceFile.mockReset();
    mocks.isBasemindPeerRefused.mockReset();
    mocks.markdownProps.length = 0;
    mocks.isBasemindPeerRefused.mockReturnValue(false);
    mocks.readWorkspaceFile.mockResolvedValue('plain source text');
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    vi.clearAllMocks();
  });

  function render(props: Partial<React.ComponentProps<typeof RedactedDocumentTabs>> = {}) {
    act(() => {
      root.render(
        <RedactedDocumentTabs
          sourcePath="/srv/workspace/deck.pptx"
          sourceFormat="pptx"
          workspaceId="workspace-1"
          {...props}
        />,
      );
    });
  }

  async function selectRedactedTab() {
    const tab = container.querySelectorAll('.redacted-document-tabs__tab')[1] as HTMLElement;
    await act(async () => {
      tab.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
  }

  it('reads the source through the workspace file owner instead of a file URL', async () => {
    render();
    await act(async () => {});

    expect(mocks.readWorkspaceFile).toHaveBeenCalledWith('workspace-1', '/srv/workspace/deck.pptx');
    expect(container.querySelector('iframe')).toBeNull();
    expect(container.querySelector('.redacted-document-tabs__source-text')?.textContent).toBe('plain source text');
  });

  it('shows the unavailable notice when the source cannot be read', async () => {
    mocks.readWorkspaceFile.mockRejectedValue(new Error('host offline'));
    render();
    await act(async () => {});

    expect(container.querySelector('.redacted-document-tabs__notice')?.textContent).toContain('state.sourceUnavailable');
    expect(container.querySelector('.redacted-document-tabs__source-text')).toBeNull();
  });

  it('counts the entities returned by the lazy load', async () => {
    mocks.readRedactedDocument.mockResolvedValue({
      markdown: 'redacted body',
      entities: [
        { entityId: 'p1', entityType: 'PERSON', occurrenceCount: 2 },
        { entityId: 'i1', entityType: 'IBAN', occurrenceCount: 1 },
      ],
      sourceFormat: 'pptx',
    });
    render();
    await selectRedactedTab();
    await act(async () => {});

    expect(mocks.readRedactedDocument).toHaveBeenCalledWith('/srv/workspace/deck.pptx', 'workspace-1');
    const counts = [...container.querySelectorAll('.redacted-document-tabs__entity-type')].map(node => node.textContent);
    expect(counts).toHaveLength(2);
    expect(counts.join(' ')).toContain('PERSON');
    expect(container.querySelector('.redacted-document-tabs__entity-count')?.textContent).toContain('3');
  });

  it('refuses the Basemind surface for a workspace on a connected device', async () => {
    mocks.isBasemindPeerRefused.mockReturnValue(true);
    render();
    await selectRedactedTab();
    await act(async () => {});

    expect(mocks.readRedactedDocument).not.toHaveBeenCalled();
    expect(container.querySelector('.redacted-document-tabs__notice')?.textContent).toContain('state.peerUnsupported');
  });

  it('enables entity reveal only for a controller-local workspace', async () => {
    mocks.readRedactedDocument.mockResolvedValue({
      markdown: 'redacted body',
      entities: [],
      sourceFormat: 'pptx',
    });
    render();
    await selectRedactedTab();
    await act(async () => {});

    expect(mocks.markdownProps.at(-1)?.enableEntityReveal).toBe(true);

    mocks.markdownProps.length = 0;
    mocks.isBasemindPeerRefused.mockReturnValue(true);
    render({ redactedMarkdown: 'redacted body' });
    await act(async () => {});

    expect(mocks.markdownProps.at(-1)?.enableEntityReveal).toBe(false);
  });

  it('shows a retryable error state when the redacted document cannot be loaded', async () => {
    mocks.readRedactedDocument.mockRejectedValue(new Error('vault locked'));
    render();
    await selectRedactedTab();
    await act(async () => {});

    const notice = container.querySelector('.redacted-document-tabs__notice');
    expect(notice?.textContent).toContain('state.loadFailed');
    expect(notice?.textContent).toContain('vault locked');
    expect(notice?.querySelector('button')).not.toBeNull();
  });
});
