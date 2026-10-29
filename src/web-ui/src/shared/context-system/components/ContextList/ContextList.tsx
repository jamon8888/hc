 

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Button, Icon, ScrollArea } from '@alex/ui';
import { AlertCircle } from 'lucide-react';
import { useContextStore, selectContexts } from '../../../stores/contextStore';
import { ContextCard } from '../ContextCard/ContextCard';
import { useI18n } from '@/infrastructure/i18n';
import { isReducedMotionPreferred } from '@/shared/utils/motionPreference';
import './ContextList.scss';

export interface ContextListProps {
  compact?: boolean;
  interactive?: boolean;
  showPreview?: boolean;
  maxHeight?: string;
  className?: string;
  onContextClick?: (contextId: string) => void;
}

const CONTEXT_ROW_EXIT_MS = 160;

export const ContextList: React.FC<ContextListProps> = ({
  compact = false,
  interactive = true,
  showPreview = true,
  maxHeight = '200px',
  className = '',
  onContextClick
}) => {
  const { t } = useI18n('components');
  const contexts = useContextStore(selectContexts);
  const removeContext = useContextStore(state => state.removeContext);
  const [leavingContextIds, setLeavingContextIds] = useState<Set<string>>(new Set());
  const exitTimersRef = useRef<Map<string, number>>(new Map());

  useEffect(() => () => {
    const pendingContextIds = [...exitTimersRef.current.keys()];
    exitTimersRef.current.forEach(timer => window.clearTimeout(timer));
    exitTimersRef.current.clear();
    pendingContextIds.forEach(removeContext);
  }, [removeContext]);

  const removeAfterExit = useCallback((contextIds: string[]) => {
    if (isReducedMotionPreferred()) {
      contextIds.forEach(removeContext);
      return;
    }

    setLeavingContextIds(previous => {
      const next = new Set(previous);
      contextIds.forEach(id => next.add(id));
      return next;
    });

    contextIds.forEach((contextId) => {
      const existingTimer = exitTimersRef.current.get(contextId);
      if (existingTimer) window.clearTimeout(existingTimer);
      const timer = window.setTimeout(() => {
        exitTimersRef.current.delete(contextId);
        removeContext(contextId);
        setLeavingContextIds(previous => {
          const next = new Set(previous);
          next.delete(contextId);
          return next;
        });
      }, CONTEXT_ROW_EXIT_MS);
      exitTimersRef.current.set(contextId, timer);
    });
  }, [removeContext]);
  
  const handleRemove = useCallback((id: string) => {
    if (!contexts.some(item => item.id === id) || leavingContextIds.has(id)) return;
    removeAfterExit([id]);
  }, [contexts, leavingContextIds, removeAfterExit]);
  
  const handleClearAll = useCallback(async () => {
    
    if (
      leavingContextIds.size === 0
      && await window.confirm(t('contextSystem.contextList.clearAllConfirm', { count: contexts.length }))
    ) {
      removeAfterExit(contexts.map(context => context.id));
    }
  }, [contexts, leavingContextIds.size, removeAfterExit, t]);
  
  const handleCardClick = useCallback((contextId: string) => {
    onContextClick?.(contextId);
  }, [onContextClick]);

  const renderedContexts = useMemo(() => {
    return contexts.map(context => ({
      context,
      exiting: leavingContextIds.has(context.id),
    }));
  }, [contexts, leavingContextIds]);
  
  if (renderedContexts.length === 0) {
    return (
      <div className={`alex-context-list alex-context-list--empty ${className}`} data-alex-component="context-list" data-alex-part="root" data-alex-state="empty">
        <div className="alex-context-list__empty-state" data-alex-component="context-list" data-alex-part="empty">
          <AlertCircle size={24} className="alex-context-list__empty-icon" />
          <p className="alex-context-list__empty-text">
            {t('contextSystem.contextList.emptyTitle')}
          </p>
          <p className="alex-context-list__empty-hint">
            {t('contextSystem.contextList.emptyHint')}
          </p>
        </div>
      </div>
    );
  }
  
  return (
    <div className={`alex-context-list ${className}`} data-alex-component="context-list" data-alex-part="root">
      
      <div className="alex-context-list__header" data-alex-component="context-list" data-alex-part="header">
        <div className="alex-context-list__title" data-alex-component="context-list" data-alex-part="title">
          {t('contextSystem.contextList.title')}
          <span className="alex-context-list__count" data-alex-component="context-list" data-alex-part="count">
            {contexts.length}
          </span>
        </div>
        
        <Button
          type="button"
          variant="outline"
          size="sm"
          leadingIcon={<Icon name="xmark" size="sm" />}
          onClick={handleClearAll}
          disabled={leavingContextIds.size > 0}
          title={t('contextSystem.contextList.clearAllTitle')}
        >
          {t('contextSystem.contextList.clearAll')}
        </Button>
      </div>
      
      
      <ScrollArea 
        className="alex-context-list__items"
        style={{ maxHeight }}
        data-alex-component="context-list"
        data-alex-part="items"
      >
        {renderedContexts.map(({ context, exiting }) => (
          <div
            key={context.id}
            className="alex-context-list__item"
            data-exiting={exiting ? 'true' : 'false'}
            aria-hidden={exiting || undefined}
            {...(exiting ? { inert: '' } : {})}
            onClick={exiting ? undefined : () => handleCardClick(context.id)}
            data-alex-component="context-list"
            data-alex-part="item"
          >
            <div className="alex-context-list__item-inner">
              <ContextCard
                context={context}
                onRemove={exiting ? undefined : handleRemove}
                compact={compact}
                interactive={interactive && !exiting}
                showPreview={showPreview}
              />
            </div>
          </div>
        ))}
      </ScrollArea>
    </div>
  );
};

export default ContextList;
