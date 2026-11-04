import { useEffect, useRef, useState } from 'react';

import { workspaceAPI } from '@/infrastructure/api/service-api/WorkspaceAPI';
import { wordcraftAPI, type WordcraftWebHostResponseDto } from '@/infrastructure/api/service-api/WordcraftAPI';
import { WordcraftBridge, type WordcraftBridgeCommand } from '@/infrastructure/api/service-api/WordcraftBridge';
import { WordcraftDocumentHost } from '@/infrastructure/api/service-api/WordcraftDocumentHost';
import { WordcraftEmbeddedDocumentSession } from '@/infrastructure/api/service-api/WordcraftEmbeddedDocumentSession';
import { useI18n } from '@/infrastructure/i18n';
import { isLocalSurface } from '@/infrastructure/peer-device/deviceSurface';
import { useEditorDocument } from '@/tools/editor/services/EditorDocument';
import {
  registerDocumentAgentContext,
  setDocumentAgentContextActive,
  unregisterDocumentAgentContext,
  updateDocumentAgentContext,
  type DocumentAgentCommandRequest,
} from '@/tools/editor/services/DocumentAgentContext';

interface WordcraftDocumentPanelProps {
  filePath: string;
  fileName: string;
  workspaceId?: string;
  surfaceId?: string;
  isActive?: boolean;
  onDirtyStateChange?: (isDirty: boolean) => void;
}

export default function WordcraftDocumentPanel({
  filePath,
  fileName,
  workspaceId,
  surfaceId = 'local',
  isActive = true,
  onDirtyStateChange,
}: WordcraftDocumentPanelProps) {
  const { t } = useI18n('components');
  const documentSession = useEditorDocument();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const isActiveRef = useRef(isActive);
  const dirtyStateChangeRef = useRef(onDirtyStateChange);
  const [surface, setSurface] = useState<WordcraftWebHostResponseDto | null>(null);
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    dirtyStateChangeRef.current = onDirtyStateChange;
  }, [onDirtyStateChange]);

  useEffect(() => {
    isActiveRef.current = isActive;
    if (!documentSession) return;
    setDocumentAgentContextActive(`wordcraft:${documentSession.id}`, isActive);
  }, [documentSession, isActive]);

  useEffect(() => {
    let active = true;
    let hostedSurface: WordcraftWebHostResponseDto | null = null;
    setSurface(null);
    setBytes(null);
    setError(null);

    async function loadDocument() {
      if (!isLocalSurface(surfaceId)) {
        setError(t('flexiblePanel.wordcraft.localOnly'));
        return;
      }
      if (!workspaceId) {
        setError(t('flexiblePanel.wordcraft.workspaceRequired'));
        return;
      }

      try {
        hostedSurface = await wordcraftAPI.hostWebSurface();
        const encoded = await workspaceAPI.readWorkspaceFile(workspaceId, filePath, 'base64');
        if (!active) {
          await wordcraftAPI.releaseWebSurface(hostedSurface.sessionId);
          return;
        }
        setBytes(decodeBase64(encoded));
        setSurface(hostedSurface);
      } catch (loadError) {
        if (hostedSurface) {
          await wordcraftAPI.releaseWebSurface(hostedSurface.sessionId).catch(() => undefined);
        }
        if (active) {
          setError(loadError instanceof Error ? loadError.message : String(loadError));
        }
      }
    }

    void loadDocument();
    return () => {
      active = false;
      if (hostedSurface) {
        void wordcraftAPI.releaseWebSurface(hostedSurface.sessionId).catch(() => undefined);
      }
    };
  }, [filePath, surfaceId, t, workspaceId]);

  useEffect(() => {
    if (!surface || !bytes) return;
    const source = iframeRef.current?.contentWindow;
    if (!source) {
      setError(t('flexiblePanel.wordcraft.frameUnavailable'));
      return;
    }

    const bridgeRef: { current: WordcraftBridge | null } = { current: null };
    const sessionRef: { current: WordcraftEmbeddedDocumentSession | null } = { current: null };
    const agentContextKey = `wordcraft:${documentSession?.id ?? filePath}`;
    const updateAgentContext = (patch: Parameters<typeof updateDocumentAgentContext>[1]) => {
      updateDocumentAgentContext(agentContextKey, patch);
    };
    const host = new WordcraftDocumentHost({
      sessionId: surface.sessionId,
      saveBytes: nextBytes => workspaceAPI.writeWorkspaceFileBase64(
        workspaceId!,
        filePath,
        encodeBase64(nextBytes),
      ),
      acknowledgeSave: (requestId, revision, ok, saveError) => {
        bridgeRef.current?.acknowledgeSave(requestId, revision, ok, saveError);
      },
      onState: state => dirtyStateChangeRef.current?.(state.dirty),
    });
    const bridge = new WordcraftBridge({
      expectedOrigin: surface.origin,
      expectedSource: source,
      expectedSessionId: surface.sessionId,
      pinnedVersion: surface.version,
      postMessage: message => source.postMessage(message, surface.origin),
      onEvent: event => {
        if (event.type === 'ready') {
          updateAgentContext({
            status: 'ready',
            revision: sessionRef.current?.state.revision ?? 0,
            capabilities: event.ready.commandCapabilities,
          });
        } else if (event.type === 'failed') {
          updateAgentContext({ status: 'failed' });
        } else if (event.type === 'dirty') {
          updateAgentContext({ revision: event.revision });
        }
        sessionRef.current?.handleBridgeEvent(event);
      },
    });
    bridgeRef.current = bridge;
    const session = new WordcraftEmbeddedDocumentSession({
      bridge,
      host,
      name: fileName,
      bytes,
      onState: state => {
        updateAgentContext({ revision: state.revision });
        dirtyStateChangeRef.current?.(state.dirty);
      },
      onError: setError,
    });
    sessionRef.current = session;
    if (documentSession) {
      registerDocumentAgentContext(
        agentContextKey,
        {
          kind: 'document',
          surface: 'wordcraft-embedded',
          documentId: documentSession.id,
          sessionId: surface.sessionId,
          surfaceId,
          workspaceId: workspaceId!,
          filePath,
          revision: session.state.revision,
          status: 'loading',
          capabilities: surface.commandCapabilities,
        },
        isActiveRef.current,
        async (request: DocumentAgentCommandRequest) => {
          if (!surface.commandCapabilities.includes(request.operation)) {
            throw new Error(`WordCraft operation '${request.operation}' is unavailable`);
          }
          const result = await bridge.request(
            request.operation as WordcraftBridgeCommand,
            request.params,
            request.expectedRevision,
            request.author,
          );
          if (!result.ok) {
            throw new Error(result.error || `WordCraft operation '${request.operation}' failed`);
          }
          return {
            value: result.value ?? null,
            revision: result.revision,
            dirty: result.dirty,
          };
        },
      );
    }
    const saveDocument = async () => {
      const result = await bridge.request('save_document', {}, session.state.revision, 'alex');
      if (!result.ok) {
        throw new Error(result.error || 'WordCraft failed to save the document');
      }
    };
    if (documentSession) {
      documentSession.save = saveDocument;
    }
    const onMessage = (event: MessageEvent<unknown>) => bridge.handleMessage(event);
    window.addEventListener('message', onMessage);

    return () => {
      window.removeEventListener('message', onMessage);
      if (documentSession?.save === saveDocument) {
        documentSession.save = undefined;
      }
      updateAgentContext({ status: 'closed' });
      unregisterDocumentAgentContext(agentContextKey);
      sessionRef.current = null;
      session.dispose();
      bridgeRef.current = null;
    };
  }, [bytes, documentSession, fileName, filePath, surface, surfaceId, t, workspaceId]);

  if (error) {
    return (
      <div className="alex-flexible-panel__loading" role="alert">
        {error}
      </div>
    );
  }
  if (!surface || !bytes) {
    return (
      <div className="alex-flexible-panel__loading">
        {t('flexiblePanel.loading.wordcraft')}
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      src={surface.url}
      title={fileName}
      aria-label={fileName}
      hidden={!isActive}
      style={{ border: 0, display: isActive ? 'block' : 'none', height: '100%', width: '100%' }}
      referrerPolicy="no-referrer"
    />
  );
}

function decodeBase64(value: string): Uint8Array {
  const binary = globalThis.atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function encodeBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return globalThis.btoa(binary);
}
