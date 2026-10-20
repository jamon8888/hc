import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Toolbar, ToolbarGroup } from '@openbitfun/ui';
import { FileText, Shield } from 'lucide-react';
import { MarkdownRenderer } from '@/infrastructure/markdown/MarkdownRenderer';
import { basemindAPI, workspaceAPI } from '@/infrastructure/api';
import type { RedactedEntity } from '@/infrastructure/api/service-api/BasemindAPI';
import { isBasemindPeerRefused } from '@/infrastructure/basemind/basemindPeerPolicy';
import { useI18n } from '@/infrastructure/i18n';
import { createLogger } from '@/shared/utils/logger';
import { basename } from 'path-browserify';
import './RedactedDocumentTabs.scss';

const log = createLogger('RedactedDocumentTabs');

interface RedactedDocumentTabsProps {
  sourcePath: string;
  redactedMarkdown?: string;
  entities?: RedactedEntity[];
  sourceFormat: string;
  workspaceId?: string;
}

interface EntityCount {
  type: string;
  count: number;
}

type LoadState = 'idle' | 'loading' | 'ready' | 'error' | 'unsupported';

/**
 * Whether retrieved content is binary rather than readable text.
 *
 * A NUL byte never appears in decoded text, so its presence means the file
 * cannot be previewed as source text.
 */
function isBinaryContent(content: string): boolean {
  return content.includes('\u0000');
}

export const RedactedDocumentTabs: React.FC<RedactedDocumentTabsProps> = ({
  sourcePath,
  redactedMarkdown,
  entities = [],
  sourceFormat,
  workspaceId,
}) => {
  const { t } = useI18n('panels/redacted-document');
  const [activeTab, setActiveTab] = useState<'source' | 'redacted'>('source');
  const [redactedContent, setRedactedContent] = useState<string | null>(redactedMarkdown ?? null);
  const [loadedEntities, setLoadedEntities] = useState<RedactedEntity[]>(entities);
  const [redactedState, setRedactedState] = useState<LoadState>(redactedMarkdown ? 'ready' : 'idle');
  const [redactedError, setRedactedError] = useState<string | null>(null);
  const [sourceContent, setSourceContent] = useState<string | null>(null);
  const [sourceState, setSourceState] = useState<LoadState>('idle');
  const redactedLoadStarted = useRef(false);
  const sourceLoadStarted = useRef(false);

  // Aggregate entities by type. The lazily loaded document owns the counts,
  // so the summary never shows the pre-load placeholder.
  const entityCounts: EntityCount[] = React.useMemo(() => {
    const counts: Record<string, number> = {};
    loadedEntities.forEach((entity) => {
      counts[entity.entityType] = (counts[entity.entityType] || 0) + entity.occurrenceCount;
    });
    return Object.entries(counts).map(([type, count]) => ({ type, count }));
  }, [loadedEntities]);

  const totalEntities = entityCounts.reduce((sum, entry) => sum + entry.count, 0);

  const loadRedactedContent = useCallback(async () => {
    if (redactedLoadStarted.current) return;
    redactedLoadStarted.current = true;
    // The Basemind surface is controller-local, so a workspace on a connected
    // device is not redactable here. Refuse before any Basemind call.
    if (isBasemindPeerRefused(workspaceId)) {
      setRedactedState('unsupported');
      return;
    }
    setRedactedState('loading');
    try {
      const result = await basemindAPI.readRedactedDocument(sourcePath, workspaceId);
      setRedactedContent(result.markdown);
      setLoadedEntities(result.entities ?? []);
      setRedactedState('ready');
    } catch (error) {
      const cause = error instanceof Error ? error : new Error(String(error));
      log.error('Failed to load redacted document', { sourcePath, workspaceId, error: cause });
      setRedactedError(cause.message);
      setRedactedState('error');
    }
  }, [sourcePath, workspaceId]);

  /**
   * Read the source through the workspace file owner.
   *
   * The host that owns the workspace serves the bytes, so a workspace on a
   * connected device reads the peer's file instead of this machine's.
   */
  const loadSourceContent = useCallback(async () => {
    if (sourceLoadStarted.current) return;
    sourceLoadStarted.current = true;
    if (!workspaceId) {
      setSourceState('unsupported');
      return;
    }
    setSourceState('loading');
    try {
      const content = await workspaceAPI.readWorkspaceFile(workspaceId, sourcePath);
      if (isBinaryContent(content)) {
        setSourceState('unsupported');
        return;
      }
      setSourceContent(content);
      setSourceState('ready');
    } catch (error) {
      const cause = error instanceof Error ? error : new Error(String(error));
      log.error('Failed to read the source document', { sourcePath, workspaceId, error: cause });
      setSourceState('unsupported');
    }
  }, [sourcePath, workspaceId]);

  useEffect(() => {
    if (activeTab === 'redacted' && !redactedContent) {
      void loadRedactedContent();
    }
  }, [activeTab, redactedContent, loadRedactedContent]);

  useEffect(() => {
    if (activeTab === 'source') {
      void loadSourceContent();
    }
  }, [activeTab, loadSourceContent]);

  const fileName = useMemo(() => basename(sourcePath), [sourcePath]);
  // The Basemind surface is controller-local, so a workspace on a connected
  // device is not redactable here. Entity reveal stays off for it.
  const peerRefused = isBasemindPeerRefused(workspaceId);

  const retryRedactedContent = useCallback(() => {
    redactedLoadStarted.current = false;
    void loadRedactedContent();
  }, [loadRedactedContent]);

  return (
    <div className="redacted-document-tabs" data-openbitfun-component="redacted-document-tabs">
      <Toolbar
        className="redacted-document-tabs__toolbar"
        leading={
          <ToolbarGroup role="tablist" aria-label={fileName}>
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'source'}
              onClick={() => setActiveTab('source')}
              className={`redacted-document-tabs__tab ${activeTab === 'source' ? 'active' : ''}`}
            >
              <FileText size={14} /> {t('tabs.source')}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'redacted'}
              onClick={() => setActiveTab('redacted')}
              className={`redacted-document-tabs__tab ${activeTab === 'redacted' ? 'active' : ''}`}
            >
              <Shield size={14} />
              {t('tabs.redacted')}
              {totalEntities > 0 && (
                <span className="redacted-document-tabs__entity-count">
                  {t('summary.piiCount', { count: totalEntities })}
                </span>
              )}
            </button>
          </ToolbarGroup>
        }
        trailing={
          <ToolbarGroup className="redacted-document-tabs__entity-summary">
            {totalEntities > 0 && (
              <span className="redacted-document-tabs__entity-badge">
                <Shield size={12} /> {t('summary.piiDetected', { count: totalEntities })}
              </span>
            )}
            {entityCounts.length > 0 && (
              <div className="redacted-document-tabs__entity-types">
                {entityCounts.map((entry) => (
                  <span
                    key={entry.type}
                    className="redacted-document-tabs__entity-type"
                  >
                    {t('entityTypeCount', { type: entry.type, count: entry.count })}
                  </span>
                ))}
              </div>
            )}
          </ToolbarGroup>
        }
      />

      <div className="redacted-document-tabs__content">
        {activeTab === 'source' ? (
          <div className="redacted-document-tabs__source">
            {sourceState === 'loading' || sourceState === 'idle' ? (
              <div className="redacted-document-tabs__loading">
                <div className="redacted-document-tabs__spinner" />
                <span>{t('state.sourceLoading')}</span>
              </div>
            ) : sourceState === 'ready' && sourceContent !== null ? (
              <>
                <div className="redacted-document-tabs__source-header">
                  <span className="redacted-document-tabs__source-name">{fileName}</span>
                  {sourceFormat && (
                    <span className="redacted-document-tabs__source-format">{sourceFormat}</span>
                  )}
                </div>
                <pre className="redacted-document-tabs__source-text">{sourceContent}</pre>
              </>
            ) : (
              <div className="redacted-document-tabs__notice">
                {t('state.sourceUnavailable', { fileName })}
              </div>
            )}
          </div>
        ) : (
          <div className="redacted-document-tabs__redacted">
            {redactedState === 'loading' || redactedState === 'idle' ? (
              <div className="redacted-document-tabs__loading">
                <div className="redacted-document-tabs__spinner" />
                <span>{t('state.loading')}</span>
              </div>
            ) : redactedState === 'unsupported' ? (
              <div className="redacted-document-tabs__notice">
                {t('state.peerUnsupported')}
              </div>
            ) : redactedState === 'error' ? (
              <div className="redacted-document-tabs__notice">
                <span>{t('state.loadFailed')}</span>
                {redactedError && (
                  <span className="redacted-document-tabs__notice-detail">{redactedError}</span>
                )}
                <button type="button" onClick={retryRedactedContent}>
                  {t('state.retry')}
                </button>
              </div>
            ) : redactedContent ? (
              <MarkdownRenderer
                content={redactedContent}
                workspaceId={workspaceId}
                enableEntityReveal={!peerRefused}
              />
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
};

export default RedactedDocumentTabs;
