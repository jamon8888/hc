 

import React, { useMemo } from 'react';
import { OverflowText, Icon, IconButton } from '@alex/ui';
import { AlertCircle, Loader2 } from 'lucide-react';
import { ContextItem } from '../../../types/context';
import { contextRegistry } from '../../../services/ContextRegistry';
import { useContextStore, selectValidationState, selectIsValidating } from '../../../stores/contextStore';
import { useI18n } from '@/infrastructure/i18n';
import './ContextCard.scss';

export interface ContextCardProps {
  context: ContextItem;
  onRemove?: (id: string) => void;
  compact?: boolean;
  interactive?: boolean;
  showPreview?: boolean;
  className?: string;
}

export const ContextCard: React.FC<ContextCardProps> = ({
  context,
  onRemove,
  compact = false,
  interactive = true,
  showPreview = true,
  className = ''
}) => {
  const { t } = useI18n('components');
  
  const validationState = useContextStore(selectValidationState(context.id));
  const isValidating = useContextStore(selectIsValidating(context.id));
  
  
  const renderer = useMemo(() => {
    return contextRegistry.getRenderer(context.type);
  }, [context.type]);
  
  
  const definition = useMemo(() => {
    return contextRegistry.getDefinition(context.type);
  }, [context.type]);
  
  
  const handleRemove = (e: React.MouseEvent) => {
    e.stopPropagation();
    onRemove?.(context.id);
  };
  
  
  const content = renderer 
    ? renderer.render(context, { compact, interactive, showPreview })
    : (
      <div className="alex-context-card__fallback">
        <div className="alex-context-card__icon">
          <AlertCircle size={20} />
        </div>
        <div className="alex-context-card__content">
          <div className="alex-context-card__title"><OverflowText>
            {t('contextSystem.contextCard.unknownType', { type: context.type })}
          </OverflowText></div>
        </div>
      </div>
    );
  
  
  const validationClass = validationState 
    ? validationState.valid 
      ? 'alex-context-card--valid'
      : 'alex-context-card--invalid'
    : '';
  
  return (
    <div 
      className={`
        alex-context-card
        alex-context-card--${context.type}
        ${validationClass}
        ${compact ? 'alex-context-card--compact' : ''}
        ${interactive ? 'alex-context-card--interactive' : ''}
        ${className}
      `.trim()}
      data-context-id={context.id}
      data-context-type={context.type}
      data-alex-component="context-list"
      data-alex-part="card"
      data-alex-state={validationState ? (validationState.valid ? 'valid' : 'invalid') : undefined}
    >
      
      {definition && (
        <div 
          className="alex-context-card__indicator"
          style={{ backgroundColor: definition.color }}
          data-alex-component="context-list"
          data-alex-part="cardIndicator"
        />
      )}
      
      
      <div className="alex-context-card__body" data-alex-component="context-list" data-alex-part="cardBody">
        {content}
      </div>
      
      
      {interactive && (
        <div className="alex-context-card__toolbar" data-alex-component="context-list" data-alex-part="cardToolbar">
          
          <div className="alex-context-card__validation">
            {isValidating ? (
              <Loader2 size={14} className="alex-context-card__spinner" />
            ) : validationState ? (
              validationState.valid ? (
                <Icon name="check-circle" size="sm" className="alex-context-card__icon--success" />
              ) : (
                <span title={validationState.error}>
                  <AlertCircle 
                    size={14} 
                    className="alex-context-card__icon--error"
                  />
                </span>
              )
            ) : null}
          </div>
          
          
          {onRemove && (
            <IconButton
              className="alex-context-card__remove-btn"
              onClick={handleRemove}
              title={t('contextSystem.contextCard.removeContext')}
              aria-label={t('contextSystem.contextCard.removeContext')}
              icon={<Icon name="xmark" size="sm" />}
            />
          )}
        </div>
      )}
      
      
      {validationState && !validationState.valid && validationState.error && (
        <div className="alex-context-card__error" data-alex-component="context-list" data-alex-part="cardError" data-alex-state="invalid">
          <AlertCircle size={12} />
          <span>{validationState.error}</span>
        </div>
      )}
      
      
      {validationState && validationState.valid && validationState.warnings && validationState.warnings.length > 0 && (
        <div className="alex-context-card__warnings">
          {validationState.warnings.map((warning, idx) => (
            <div key={idx} className="alex-context-card__warning">
              <AlertCircle size={12} />
              <span>{warning}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ContextCard;
