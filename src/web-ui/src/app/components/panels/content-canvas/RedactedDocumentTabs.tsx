import React, { useState, useEffect, useCallback } from 'react';
import { Toolbar, ToolbarGroup, ToolbarItem } from '@openbitfun/ui';
import { FileText, Lock, Eye, EyeOff, Shield } from 'lucide-react';
import { MarkdownRenderer } from '@/infrastructure/markdown/MarkdownRenderer';
import { basemindAPI } from '@/infrastructure/api';
import { basename } from 'path-browserify';
import './RedactedDocumentTabs.scss';

interface RedactedDocumentTabsProps {
  sourcePath: string;
  redactedMarkdown?: string;
  entities?: Array<{ entityId: string; entityType: string; occurrenceCount: number }>;
  sourceFormat: string;
  workspaceId?: string;
}

interface EntityCount {
  type: string;
  count: number;
}

export const RedactedDocumentTabs: React.FC<RedactedDocumentTabsProps> = ({
  sourcePath,
  redactedMarkdown,
  entities = [],
  sourceFormat,
  workspaceId,
}) => {
  const [activeTab, setActiveTab] = useState<'source' | 'redacted'>('source');
  const [redactedContent, setRedactedContent] = useState<string | null>(redactedMarkdown ?? null);
  const [isLoading, setIsLoading] = useState(!redactedMarkdown);

  // Aggregate entities by type
  const entityCounts: EntityCount[] = React.useMemo(() => {
    const counts: Record<string, number> = {};
    entities.forEach((e) => {
      counts[e.entityType] = (counts[e.entityType] || 0) + e.occurrenceCount;
    });
    return Object.entries(counts).map(([type, count]) => ({ type, count }));
  }, [entities]);

  const totalEntities = entityCounts.reduce((sum, e) => sum + e.count, 0);

  const loadRedactedContent = useCallback(async () => {
    if (redactedContent) return;
    setIsLoading(true);
    try {
      const result = await basemindAPI.readRedactedDocument(sourcePath, workspaceId);
      setRedactedContent(result.markdown);
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

  return (
    <div className="redacted-document-tabs" data-openbitfun-component="redacted-document-tabs">
      <Toolbar className="redacted-document-tabs__toolbar">
        <ToolbarGroup>
          <ToolbarItem
            role="tab"
            aria-selected={activeTab === 'source'}
            onClick={() => setActiveTab('source')}
            className={`redacted-document-tabs__tab ${activeTab === 'source' ? 'active' : ''}`}
          >
            <FileText size={14} /> Source
          </ToolbarItem>
          <ToolbarItem
            role="tab"
            aria-selected={activeTab === 'redacted'}
            onClick={() => setActiveTab('redacted')}
            className={`redacted-document-tabs__tab ${activeTab === 'redacted' ? 'active' : ''}`}
          >
            <Shield size={14} />
            Redacted
            {totalEntities > 0 && (
              <span className="redacted-document-tabs__entity-count">
                ({totalEntities} PII)
              </span>
            )}
          </ToolbarItem>
        </ToolbarGroup>

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
      </Toolbar>

      <div className="redacted-document-tabs__content">
        {activeTab === 'source' ? (
          <div className="redacted-document-tabs__source">
            <iframe
              className="redacted-document-tabs__iframe"
              src={`file://${sourcePath}`}
              title={`Source: ${fileName}`}
            />
          </div>
        ) : (
          <div className="redacted-document-tabs__redacted">
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