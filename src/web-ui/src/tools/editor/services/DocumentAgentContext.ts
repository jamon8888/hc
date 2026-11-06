import { resourcePathKey } from '@/shared/utils/resourcePath';
import type { ContentResourceScope } from '@/shared/types/contentResource';

/**
 * Generic context for a document surface that an agent may be asked to edit.
 *
 * The composer only consumes this contract. Surface-specific control APIs stay
 * in the owning presenter (for example, the embedded WordCraft panel).
 */
export interface DocumentAgentContext {
  kind: 'document';
  surface: 'wordcraft-embedded';
  documentId: string;
  sessionId: string;
  surfaceId: string;
  workspaceId: string;
  workspacePath?: string;
  remoteConnectionId?: string;
  filePath: string;
  revision: number;
  status: 'loading' | 'ready' | 'failed' | 'closed';
  capabilities: string[];
}

export interface DocumentAgentCommandRequest {
  operation: string;
  params: Record<string, unknown>;
  expectedRevision: number;
  author: string;
}

export type DocumentAgentCommandExecutor = (
  request: DocumentAgentCommandRequest,
) => Promise<unknown>;

interface RegisteredContext {
  context: DocumentAgentContext;
  active: boolean;
  sequence: number;
  executor?: DocumentAgentCommandExecutor;
}

const registeredContexts = new Map<string, RegisteredContext>();
let sequence = 0;

/** Register or refresh one document context. Only active presenters are eligible. */
export function registerDocumentAgentContext(
  key: string,
  context: DocumentAgentContext,
  active: boolean,
  executor?: DocumentAgentCommandExecutor,
): void {
  registeredContexts.set(key, {
    context,
    active,
    sequence: ++sequence,
    executor,
  });
}

/** Update a registered context without allowing callers to replace its identity. */
export function updateDocumentAgentContext(
  key: string,
  patch: Partial<Omit<DocumentAgentContext, 'kind' | 'surface' | 'documentId' | 'sessionId' | 'surfaceId' | 'workspaceId' | 'workspacePath' | 'remoteConnectionId' | 'filePath'>>,
): void {
  const registered = registeredContexts.get(key);
  if (!registered) return;
  registeredContexts.set(key, {
    ...registered,
    context: { ...registered.context, ...patch },
    sequence: registered.sequence,
  });
}

/** Change whether a registered surface is the active composer target. */
export function setDocumentAgentContextActive(key: string, active: boolean): void {
  const registered = registeredContexts.get(key);
  if (!registered) return;
  registeredContexts.set(key, {
    ...registered,
    active,
    sequence: active ? ++sequence : registered.sequence,
  });
}

export function unregisterDocumentAgentContext(key: string): void {
  registeredContexts.delete(key);
}

/**
 * Return the embedded owner of a file, regardless of which tab is active.
 * Opening the same file in the native WordCraft window would create a second
 * writer and bypass the embedded session's revision guard.
 */
export function getDocumentAgentContextForFile(
  filePath: string,
  scopeOrWorkspaceId?: ContentResourceScope | string,
  surfaceId?: string,
): DocumentAgentContext | undefined {
  const requestedScope: ContentResourceScope | undefined =
    typeof scopeOrWorkspaceId === 'string'
      ? { workspaceId: scopeOrWorkspaceId, surfaceId: surfaceId ?? '' }
      : scopeOrWorkspaceId;
  return [...registeredContexts.values()].find(
    registered =>
      registered.context.status !== 'closed' &&
      (!requestedScope?.workspaceId ||
        registered.context.workspaceId === requestedScope.workspaceId) &&
      (!requestedScope?.surfaceId ||
        registered.context.surfaceId === requestedScope.surfaceId) &&
      (!requestedScope?.remoteConnectionId ||
        registered.context.remoteConnectionId === requestedScope.remoteConnectionId) &&
      resourcePathKey(
        registered.context.filePath,
        contextScope(registered.context),
      ) === resourcePathKey(
        filePath,
        requestedScope ?? contextScope(registered.context),
      ),
  )?.context;
}

function contextScope(context: DocumentAgentContext): ContentResourceScope {
  return {
    surfaceId: context.surfaceId,
    workspaceId: context.workspaceId,
    workspacePath: context.workspacePath,
    remoteConnectionId: context.remoteConnectionId,
  };
}

/** Execute a request only on the exact visible document surface named by Core. */
export async function executeDocumentAgentCommand(
  documentId: string,
  sessionId: string,
  request: DocumentAgentCommandRequest,
): Promise<unknown> {
  const registered = [...registeredContexts.values()].find(
    candidate =>
      candidate.active &&
      candidate.context.documentId === documentId &&
      candidate.context.sessionId === sessionId,
  );
  if (!registered) {
    throw new Error('The requested embedded document surface is closed or inactive');
  }
  if (registered.context.status !== 'ready') {
    throw new Error(`The embedded document surface is not ready (${registered.context.status})`);
  }
  if (!registered.executor) {
    throw new Error('The embedded document surface has no command controller');
  }
  return registered.executor(request);
}

/** Return the most recently activated context for the current composer. */
export function getActiveDocumentAgentContext(): DocumentAgentContext | undefined {
  return [...registeredContexts.values()]
    .filter(registered => registered.active)
    .sort((left, right) => right.sequence - left.sequence)[0]?.context;
}

export function clearDocumentAgentContextsForTests(): void {
  registeredContexts.clear();
  sequence = 0;
}
