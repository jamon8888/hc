import { OverflowText } from '@alex/ui';
/**
 * Generic panel header with centered title and optional action buttons.
 */

import React from 'react';
import './PanelHeader.scss';

export interface PanelHeaderProps {
  title: string;
  actions?: React.ReactNode;
  className?: string;
}

export const PanelHeader: React.FC<PanelHeaderProps> = ({
  title,
  actions,
  className = '',
}) => {
  return (
    <div data-alex-component="panel-header" data-alex-part="root" className={`alex-panel-header ${className}`}>
      <h3 className="alex-panel-header__title" data-alex-component="panel-header" data-alex-part="title"><OverflowText>{title}</OverflowText></h3>
      {actions && (
        <div className="alex-panel-header__actions" data-alex-component="panel-header" data-alex-part="actions">
          {actions}
        </div>
      )}
    </div>
  );
};

PanelHeader.displayName = 'PanelHeader';

export default PanelHeader;
