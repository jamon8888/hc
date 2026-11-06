import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  clearDocumentAgentContextsForTests,
  executeDocumentAgentCommand,
  getActiveDocumentAgentContext,
  registerDocumentAgentContext,
  setDocumentAgentContextActive,
  unregisterDocumentAgentContext,
  updateDocumentAgentContext,
} from './DocumentAgentContext';

const context = (documentId: string) => ({
  kind: 'document' as const,
  surface: 'wordcraft-embedded' as const,
  documentId,
  sessionId: `session-${documentId}`,
  surfaceId: 'local',
  workspaceId: 'workspace-1',
  filePath: `/workspace/${documentId}.docx`,
  revision: 0,
  status: 'loading' as const,
  capabilities: ['list_commands'],
});

describe('DocumentAgentContext', () => {
  beforeEach(() => clearDocumentAgentContextsForTests());

  it('returns only the most recently activated document', () => {
    registerDocumentAgentContext('first', context('first'), true);
    registerDocumentAgentContext('second', context('second'), false);
    expect(getActiveDocumentAgentContext()?.documentId).toBe('first');

    setDocumentAgentContextActive('second', true);
    expect(getActiveDocumentAgentContext()?.documentId).toBe('second');
  });

  it('updates lifecycle state without changing document identity', () => {
    registerDocumentAgentContext('doc', context('doc'), true);
    updateDocumentAgentContext('doc', { status: 'ready', revision: 3 });

    expect(getActiveDocumentAgentContext()).toMatchObject({
      documentId: 'doc',
      status: 'ready',
      revision: 3,
    });
  });

  it('does not let revision updates steal the active document selection', () => {
    registerDocumentAgentContext('first', context('first'), true);
    registerDocumentAgentContext('second', context('second'), true);
    updateDocumentAgentContext('first', { revision: 4 });

    expect(getActiveDocumentAgentContext()?.documentId).toBe('second');
  });

  it('routes a command only to the matching ready surface', async () => {
    const execute = vi.fn().mockResolvedValue({ revision: 2 });
    registerDocumentAgentContext('doc', { ...context('doc'), status: 'ready' }, true, execute);

    await expect(
      executeDocumentAgentCommand('doc', 'session-doc', {
        operation: 'get_text',
        params: {},
        expectedRevision: 1,
        author: 'Alex Agent (agent-1)',
      }),
    ).resolves.toEqual({ revision: 2 });
    expect(execute).toHaveBeenCalledWith({
      operation: 'get_text',
      params: {},
      expectedRevision: 1,
      author: 'Alex Agent (agent-1)',
    });
  });

  it('removes a closed surface from the active target', () => {
    registerDocumentAgentContext('doc', context('doc'), true);
    unregisterDocumentAgentContext('doc');
    expect(getActiveDocumentAgentContext()).toBeUndefined();
  });
});
