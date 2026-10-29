import React from 'react';

import { useTranslation } from 'react-i18next';
import { IconButton, Tooltip, Icon, OverflowText } from '@alex/ui';
import { useCopyTextAction } from '../hooks/useCopyTextAction';
import './CopyableTextPreview.scss';

interface CopyableTextPreviewProps extends React.HTMLAttributes<HTMLElement> {
  text?: string | null;
  emptyText: React.ReactNode;
  as?: 'span' | 'code';
  /** Preserve full resource details instead of using single-line overflow. */
  multiline?: boolean;
  className?: string;
  tooltipContent?: React.ReactNode;
  tooltipPlacement?: 'top' | 'bottom' | 'left' | 'right';
}

export const CopyableTextPreview = React.forwardRef<HTMLElement, CopyableTextPreviewProps>(({
  text,
  emptyText,
  as = 'span',
  multiline = false,
  className,
  tooltipContent,
  tooltipPlacement = 'bottom',
  ...restProps
}, ref) => {
  const { t } = useTranslation('flow-chat');
  const content = text?.trim()
    ? text
    : <span className="copyable-text-preview__empty" data-alex-component="copyable-text-preview" data-alex-part="empty">{emptyText}</span>;
  const resolvedClassName = `copyable-text-preview${multiline ? ' copyable-text-preview--multiline' : ''}${className ? ` ${className}` : ''}`;
  const preview = multiline ? content : (
    <OverflowText title={tooltipContent ? '' : undefined}>{content}</OverflowText>
  );
  const copyText = typeof tooltipContent === 'string' && tooltipContent.trim()
    ? tooltipContent
    : undefined;
  const { copied, copy } = useCopyTextAction({
    getText: () => copyText ?? '',
    successMessage: t('toolCards.common.copied'),
    failureMessage: t('toolCards.common.copyFailed'),
    showSuccessNotification: false,
  });
  const copyTooltip = copied ? t('toolCards.common.copied') : t('toolCards.common.copy');
  const node = as === 'code' ? (
    <code ref={ref} className={resolvedClassName} {...restProps} data-alex-component="copyable-text-preview" data-alex-part="root">
      {preview}
    </code>
  ) : (
    <span ref={ref} className={resolvedClassName} {...restProps} data-alex-component="copyable-text-preview" data-alex-part="root">
      {preview}
    </span>
  );

  if (!tooltipContent) {
    return node;
  }

  return (
    <Tooltip
      content={
        <div className="copyable-text-preview-tooltip-content" data-alex-component="copyable-text-preview" data-alex-part="tooltipContent">
          <span className="copyable-text-preview-tooltip-content__text" data-alex-component="copyable-text-preview" data-alex-part="tooltipText">{tooltipContent}</span>
          {copyText && (
            <Tooltip content={copyTooltip}>
              <IconButton
                className={`copyable-text-preview-tooltip__copy${copied ? ' copied' : ''}`}
                data-alex-component="copyable-text-preview"
                data-alex-part="copyAction"
                data-alex-state={copied ? 'copied' : undefined}
                variant="quiet"
                size="xs"
                onClick={copy}
                icon={copied ? <Icon name="check-line" size="xs" /> : <Icon name="duplicate" size="xs" />}
                aria-label={copyTooltip}
              />
            </Tooltip>
          )}
        </div>
      }
      placement={tooltipPlacement}
      className="copyable-text-preview-tooltip"
      interactive
    >
      {node}
    </Tooltip>
  );
});

CopyableTextPreview.displayName = 'CopyableTextPreview';
