import React, { useState, useEffect, useCallback } from 'react';
import { Toolbar, ToolbarGroup } from '@openbitfun/ui';
import { FileText, Shield } from 'lucide-react';
import { MarkdownRenderer } from '@/infrastructure/markdown/MarkdownRenderer';
import { basemindAPI, type RedactedEntity } from '@/infrastructure/api';
import { basename } from 'path-browserify';
import './RedactedDocumentTabs.scss';

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

/** A redacted body fetched by this component, tagged with its origin document. */
interface FetchedBody {
  key: string;
  markdown: string;
  entities?: RedactedEntity[];
}

/** Formats a browser will not render inline, so an iframe preview is pointless. */
const OFFICE_SOURCE_FORMATS = new Set([
  'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'odt', 'ods', 'odp', 'rtf',
]);

export const RedactedDocumentTabs: React.FC<RedactedDocumentTabsProps> = ({
  sourcePath,
  redactedMarkdown,
  entities,
  sourceFormat,
  workspaceId,
}) => {
  const [activeTab, setActiveTab] = useState<'source' | 'redacted'>('source');
  const [isLoading, setIsLoading] = useState(!redactedMarkdown);

  // Key the fetched body to the document it was produced from. Without this,
  // switching tabs or files would keep rendering the previous document's
  // (possibly sensitive) content under the new header.
  const documentKey = `${sourcePath}::${workspaceId ?? ''}`;
  const [fetchedBody, setFetchedBody] = useState<FetchedBody | null>(null);

  // Only a body belonging to the currently displayed document may be rendered.
  const matchedBody = fetchedBody?.key === documentKey ? fetchedBody : null;
  const redactedContent = redactedMarkdown ?? matchedBody?.markdown ?? null;

  // Entities supplied by a caller win; otherwise use what the fetch returned
  // for this document, so the PII counter is not permanently zero. Memoized so
  // the empty-array fallback does not become a new identity on every render.
  const activeEntities = React.useMemo(
    () => entities ?? matchedBody?.entities ?? [],
    [entities, matchedBody],
  );

  // Aggregate entities by type
  const entityCounts: EntityCount[] = React.useMemo(() => {
    const counts: Record<string, number> = {};
    activeEntities.forEach((e) => {
      counts[e.entityType] = (counts[e.entityType] || 0) + e.occurrenceCount;
    });
    return Object.entries(counts).map(([type, count]) => ({ type, count }));
  }, [activeEntities]);

  const totalEntities = entityCounts.reduce((sum, e) => sum + e.count, 0);

  const loadRedactedContent = useCallback(async () => {
    if (redactedContent) return;
    setIsLoading(true);
    try {
      const result = await basemindAPI.readRedactedDocument(sourcePath, workspaceId);
      // Keying by document means a late response for a previously viewed file
      // can never be rendered against the one on screen.
      setFetchedBody({
        key: `${sourcePath}::${workspaceId ?? ''}`,
        markdown: result.markdown,
        entities: result.entities,
      });
    } catch (error) {
      console.error('Failed to load redacted document:', error);
    } finally {
      setIsLoading(false);
    }
  }, [sourcePath, workspaceId, redactedContent]);

  useEffect(() => {
    if (activeTab === 'redacted' && !redactedContent) {
      loadRedactedContent();
    }
  }, [activeTab, redactedContent, loadRedactedContent]);

  const formatFileName = (path: string) => basename(path);
  const fileName = formatFileName(sourcePath);

  const tabId = React.useId();

  // Roving focus between the two tabs, per the ARIA tabs pattern.
  const handleTabKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
      event.preventDefault();
      setActiveTab((current) => (current === 'source' ? 'redacted' : 'source'));
    },
    [],
  );

  // Browsers cannot render Office documents in an iframe, so the source pane
  // falls back to a message for those formats instead of showing a blank frame.
  const sourceExtension = (sourceFormat || fileName.split('.').pop() || '').toLowerCase();
  const supportsIframePreview = !OFFICE_SOURCE_FORMATS.has(sourceExtension);

  return (
    <div className="redacted-document-tabs" data-openbitfun-component="redacted-document-tabs" data-openbitfun-part="redacted-document-tabs">
      <Toolbar
        className="redacted-document-tabs__toolbar"
        leading={
          <ToolbarGroup role="tablist" aria-label={fileName}>
            <button
              type="button"
              role="tab"
              id={`${tabId}-tab-source`}
              aria-selected={activeTab === 'source'}
              aria-controls={`${tabId}-panel-source`}
              tabIndex={activeTab === 'source' ? 0 : -1}
              onClick={() => setActiveTab('source')}
              onKeyDown={handleTabKeyDown}
              className={`redacted-document-tabs__tab ${activeTab === 'source' ? 'active' : ''}`}
            >
              <FileText size={14} /> Source
            </button>
            <button
              type="button"
              role="tab"
              id={`${tabId}-tab-redacted`}
              aria-selected={activeTab === 'redacted'}
              aria-controls={`${tabId}-panel-redacted`}
              tabIndex={activeTab === 'redacted' ? 0 : -1}
              onClick={() => setActiveTab('redacted')}
              onKeyDown={handleTabKeyDown}
              className={`redacted-document-tabs__tab ${activeTab === 'redacted' ? 'active' : ''}`}
            >
              <Shield size={14} />
              Redacted
              {totalEntities > 0 && (
                <span className="redacted-document-tabs__entity-count">
                  ({totalEntities} PII)
                </span>
              )}
            </button>
          </ToolbarGroup>
        }
        trailing={
          <ToolbarGroup className="redacted-document-tabs__entity-summary">
            {totalEntities > 0 && (
              <span className="redacted-document-tabs__entity-badge">
                <Shield size={12} /> {totalEntities} PII detected
              </span>
            )}
            {entityCounts.length > 0 && (
              <div className="redacted-document-tabs__entity-types">
                {entityCounts.map((e) => (
                  <span
                    key={e.type}
                    className="redacted-document-tabs__entity-type"
                    title={`${e.type}: ${e.count}`}
                  >
                    {e.type}: {e.count}
                  </span>
                ))}
              </div>
            )}
          </ToolbarGroup>
        }
      />

      <div className="redacted-document-tabs__content">
        {activeTab === 'source' ? (
          <div
            className="redacted-document-tabs__source"
            id={`${tabId}-panel-source`}
            role="tabpanel"
            aria-labelledby={`${tabId}-tab-source`}
          >
            {supportsIframePreview ? (
              <iframe
                className="redacted-document-tabs__iframe"
                src={`file://${sourcePath}`}
                title={`Source: ${fileName}`}
              />
            ) : (
              <div className="redacted-document-tabs__unsupported">
                <FileText size={20} />
                <span>
                  {fileName} cannot be previewed inline. Open it in an external
                  application, or use the Redacted tab for the extracted text.
                </span>
              </div>
            )}
          </div>
        ) : (
          <div
            className="redacted-document-tabs__redacted"
            id={`${tabId}-panel-redacted`}
            role="tabpanel"
            aria-labelledby={`${tabId}-tab-redacted`}
          >
            {isLoading ? (
              <div className="redacted-document-tabs__loading">
                <div className="spinner" />
                <span>Loading redacted document...</span>
              </div>
            ) : redactedContent ? (
              <MarkdownRenderer
                content={redactedContent}
                workspaceId={workspaceId}
              />
            ) : (
              <div className="redacted-document-tabs__error">
                Failed to load redacted document.
                <button onClick={loadRedactedContent}>Retry</button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default RedactedDocumentTabs;