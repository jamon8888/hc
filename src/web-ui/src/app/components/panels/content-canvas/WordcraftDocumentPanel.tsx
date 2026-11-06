import { useEffect, useRef, useState } from 'react';

import { workspaceAPI } from '@/infrastructure/api/service-api/WorkspaceAPI';
import {
  wordcraftAPI,
  type WordcraftCapabilityDto,
  type WordcraftWebHostResponseDto,
} from '@/infrastructure/api/service-api/WordcraftAPI';
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
  workspacePath?: string;
  surfaceId?: string;
  remoteConnectionId?: string;
  isActive?: boolean;
  onDirtyStateChange?: (isDirty: boolean) => void;
}

export default function WordcraftDocumentPanel({
  filePath,
  fileName,
  workspaceId,
  workspacePath,
  surfaceId,
  remoteConnectionId,
  isActive = true,
  onDirtyStateChange,
}: WordcraftDocumentPanelProps) {
  const { t } = useI18n('components');
  const documentSession = useEditorDocument();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const disposeEmbeddedRef = useRef<((timeoutMs?: number) => Promise<void>) | null>(null);
  const disposalBarrierRef = useRef<Promise<void>>(Promise.resolve());
  const releasedSurfaceRef = useRef<string | null>(null);
  const dirtyStateChangeRef = useRef(onDirtyStateChange);
  const isActiveRef = useRef(isActive);
  const translateRef = useRef(t);
  const currentFilePathRef = useRef(filePath);
  const preservedRenameRef = useRef<{ sourcePath: string; bytes: Uint8Array; expectedHash?: string } | null>(null);
  currentFilePathRef.current = filePath;
  const [surface, setSurface] = useState<WordcraftWebHostResponseDto | null>(null);
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fallbackCapability, setFallbackCapability] = useState<WordcraftCapabilityDto | null>(null);
  const [fallbackState, setFallbackState] = useState<'idle' | 'opening' | 'opened' | 'failed'>('idle');
  const [fallbackError, setFallbackError] = useState<string | null>(null);

  useEffect(() => {
    dirtyStateChangeRef.current = onDirtyStateChange;
  }, [onDirtyStateChange]);

  useEffect(() => {
    translateRef.current = t;
  }, [t]);

  useEffect(() => {
    isActiveRef.current = isActive;
    if (!documentSession) return;
    setDocumentAgentContextActive(`wordcraft:${documentSession.id}`, isActive);
  }, [documentSession, isActive]);

  useEffect(() => {
    let active = true;
    let hostedSurface: WordcraftWebHostResponseDto | null = null;
    releasedSurfaceRef.current = null;
    setSurface(null);
    setBytes(null);
    setError(null);
    setFallbackCapability(null);
    setFallbackState('idle');
    setFallbackError(null);

    async function loadDocument() {
      try {
        await disposalBarrierRef.current;
        if (!active) return;
        if (!surfaceId) {
          setError(t('flexiblePanel.wordcraft.surfaceRequired'));
          return;
        }
        if (remoteConnectionId) {
          setError(t('flexiblePanel.wordcraft.localOnly'));
          return;
        }
        if (!isLocalSurface(surfaceId)) {
          setError(translateRef.current('flexiblePanel.wordcraft.localOnly'));
          return;
        }
        if (!workspaceId) {
          setError(translateRef.current('flexiblePanel.wordcraft.workspaceRequired'));
          return;
        }
        hostedSurface = await wordcraftAPI.hostWebSurface();
        const preserved = preservedRenameRef.current;
        if (preserved && preserved.sourcePath !== filePath) {
          if (!active) {
            // This panel will not consume the preserved rename bytes; clear
            // them so a later load cannot splice foreign content.
            preservedRenameRef.current = null;
            await wordcraftAPI.releaseWebSurface(hostedSurface.sessionId);
            return;
          }
          setBytes(new Uint8Array(preserved.bytes));
          setSurface(hostedSurface);
          return;
        }
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
      if (hostedSurface && releasedSurfaceRef.current !== hostedSurface.sessionId) {
        void wordcraftAPI.releaseWebSurface(hostedSurface.sessionId).catch(() => undefined);
      }
    };
  }, [filePath, remoteConnectionId, surfaceId, workspaceId]);

  useEffect(() => {
    const embeddedSurfaceId = surfaceId;
    if (remoteConnectionId || !error || !workspaceId || !embeddedSurfaceId || !isLocalSurface(embeddedSurfaceId)) return;
    let active = true;
    void wordcraftAPI.statusQuiet().then(capability => {
      if (active) setFallbackCapability(capability);
    });
    return () => {
      active = false;
    };
  }, [error, remoteConnectionId, surfaceId, workspaceId]);

  useEffect(() => {
    const embeddedSurfaceId = surfaceId;
    if (!surface || !bytes || !embeddedSurfaceId) return;
    const source = iframeRef.current?.contentWindow;
    if (!source) {
      setError(translateRef.current('flexiblePanel.wordcraft.frameUnavailable'));
      return;
    }

    const bridgeRef: { current: WordcraftBridge | null } = { current: null };
    const sessionRef: { current: WordcraftEmbeddedDocumentSession | null } = { current: null };
    const preserved = preservedRenameRef.current;
    const preservedHash = preserved && preserved.sourcePath !== filePath
      ? preserved.expectedHash
      : undefined;
    const initialHashPromise = preservedHash
      ? Promise.resolve(preservedHash)
      : sha256Hex(bytes);
    if (preserved?.sourcePath !== filePath) preservedRenameRef.current = null;
    let expectedHash: string | undefined;
    const agentContextKey = `wordcraft:${documentSession?.id ?? filePath}`;
    const updateAgentContext = (patch: Parameters<typeof updateDocumentAgentContext>[1]) => {
      updateDocumentAgentContext(agentContextKey, patch);
    };
    const host = new WordcraftDocumentHost({
      sessionId: surface.sessionId,
      saveBytes: async nextBytes => {
        const currentHash = expectedHash ?? await initialHashPromise;
        expectedHash = currentHash;
        await workspaceAPI.writeWorkspaceFileBase64(
          workspaceId!,
          filePath,
          encodeBase64(nextBytes),
          currentHash,
        );
        expectedHash = await sha256Hex(nextBytes);
      },
      acknowledgeSave: (requestId, revision, ok, saveError) => {
        bridgeRef.current?.acknowledgeSave(requestId, revision, ok, saveError);
      },
      onState: state => dirtyStateChangeRef.current?.(state.dirty),
    }, bytes);
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
          surfaceId: embeddedSurfaceId,
          workspaceId: workspaceId!,
          workspacePath,
          remoteConnectionId,
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
          if (request.operation === 'save_document') {
            await host.waitForDurableSave(result.revision);
            const persisted = await workspaceAPI.readWorkspaceFile(workspaceId!, filePath, 'base64');
            if (typeof persisted !== 'string' || persisted.length === 0) {
              throw new Error('WordCraft durable save read-back was empty');
            }
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
      await host.waitForDurableSave(result.revision);
    };
    if (documentSession) {
      documentSession.save = saveDocument;
    }
    void initialHashPromise
      .then(hash => {
        expectedHash = hash;
      })
      .catch(hashError => {
        setError(hashError instanceof Error ? hashError.message : String(hashError));
      });

    const onMessage = (event: MessageEvent<unknown>) => bridge.handleMessage(event);
    window.addEventListener('message', onMessage);

    let embeddedDisposed = false;
    let disposalPromise: Promise<void> | null = null;
    const disposeEmbedded = () => {
      if (disposalPromise) return disposalPromise;
      disposalPromise = (async () => {
        if (embeddedDisposed) return;
        embeddedDisposed = true;
        window.removeEventListener('message', onMessage);
        if (documentSession?.save === saveDocument) {
          documentSession.save = undefined;
        }
        updateAgentContext({ status: 'closed' });
        if (host.state.dirty && currentFilePathRef.current !== filePath) {
          let sourceHash = expectedHash;
          if (!sourceHash) {
            try {
              sourceHash = await initialHashPromise;
            } catch {
              sourceHash = undefined;
            }
          }
          preservedRenameRef.current = {
            sourcePath: filePath,
            bytes: host.getLatestBytes(bytes),
            expectedHash: sourceHash,
          };
        }
        unregisterDocumentAgentContext(agentContextKey);
        sessionRef.current = null;
        await session.dispose();
        bridgeRef.current = null;
      })();
      return disposalPromise;
    };
    disposeEmbeddedRef.current = disposeEmbedded;

    return () => {
      if (disposeEmbeddedRef.current === disposeEmbedded) {
        disposeEmbeddedRef.current = null;
      }
      const disposal = disposeEmbedded();
      disposalBarrierRef.current = disposal;
      void disposal;
    };
  }, [bytes, documentSession, fileName, filePath, surface, surfaceId, workspaceId, workspacePath]);

  if (error) {
    const nativeFallbackAvailable =
      fallbackCapability?.status === 'available' && Boolean(fallbackCapability.app);
    const fallbackDetail =
      fallbackError ||
      fallbackCapability?.remediation ||
      fallbackCapability?.message ||
      (!nativeFallbackAvailable ? translateRef.current('flexiblePanel.wordcraft.nativeUnavailable') : null);
    const openNativeFallback = async () => {
      setFallbackState('opening');
      setFallbackError(null);
      try {
        const disposal = disposeEmbeddedRef.current?.() ?? disposalBarrierRef.current;
        await withTimeout(disposal, 10_000);
        setSurface(null);
        if (surface) {
          releasedSurfaceRef.current = surface.sessionId;
          await wordcraftAPI.releaseWebSurface(surface.sessionId);
        }
        await wordcraftAPI.openDocument({ path: filePath, workspaceId });
        setFallbackState('opened');
      } catch (fallbackOpenError) {
        setFallbackState('failed');
        setFallbackError(
          fallbackOpenError instanceof Error ? fallbackOpenError.message : String(fallbackOpenError),
        );
      }
    };
    return (
      <div className="alex-flexible-panel__loading" role="alert">
        <div>{error}</div>
        {fallbackDetail && <div role="status">{fallbackDetail}</div>}
        {surface && fallbackState !== 'opened' && (
          <button
            type="button"
            onClick={() => {
              setError(null);
              setFallbackError(null);
              if (iframeRef.current) iframeRef.current.src = surface.url;
            }}
            data-testid="wordcraft-retry"
          >
            {translateRef.current('error.retry')}
          </button>
        )}
        {!remoteConnectionId && nativeFallbackAvailable && fallbackState !== 'opened' && (
          <button
            type="button"
            onClick={() => void openNativeFallback()}
            disabled={fallbackState === 'opening'}
            data-testid="wordcraft-fallback"
            data-alex-state={fallbackState}
          >
            {fallbackState === 'opening'
              ? translateRef.current('flexiblePanel.wordcraft.openingNative')
              : translateRef.current('flexiblePanel.wordcraft.openNative')}
          </button>
        )}
        {fallbackState === 'opened' && (
          <div role="status">{translateRef.current('flexiblePanel.wordcraft.nativeOpened')}</div>
        )}
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
    <div style={{ height: '100%', position: 'relative', width: '100%' }}>
      <iframe
        ref={iframeRef}
        src={surface.url}
        title={fileName}
        aria-label={fileName}
        hidden={!isActive}
        style={{ border: 0, display: isActive ? 'block' : 'none', height: '100%', width: '100%' }}
        referrerPolicy="no-referrer"
      />
      {error && (
        <div className="alex-flexible-panel__loading" role="alert">
          <div>{error}</div>
          <button
            type="button"
            onClick={() => {
              setError(null);
              if (iframeRef.current) {
                iframeRef.current.src = surface.url;
              }
            }}
          >
            {translateRef.current('error.retry')}
          </button>
        </div>
      )}
    </div>
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

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => {
      timer = setTimeout(() => reject(new Error('WordCraft document teardown timed out')), timeoutMs);
    }),
  ]).finally(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  if (typeof crypto === 'undefined' || !crypto.subtle?.digest) {
    throw new Error('WordCraft save protection requires crypto.subtle.digest');
  }
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest))
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('');
}
