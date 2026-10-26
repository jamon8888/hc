import React from 'react';
import { PageHeader } from '@alex/ui';
import { formatStandaloneUiText } from './standaloneUiCopy';
import './ConfigPageHeader.scss';

export interface ConfigPageHeaderProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'title'> {
  title: string;
  subtitle?: string;
  icon?: React.ReactNode;
  extra?: React.ReactNode;
  className?: string;
}

export const ConfigPageHeader: React.FC<ConfigPageHeaderProps> = ({
  title,
  subtitle,
  icon: _icon,
  extra,
  className = '',
  ...props
}) => {
  return (
    <div className={`alex-config-page-header ${className}`} data-alex-component="config" data-alex-part="pageHeader" {...props}>
      <div className="alex-config-page-header__inner" data-alex-component="config" data-alex-part="pageHeaderInner">
        <div className="alex-config-page-header__left">
          <div className="alex-config-page-header__info" data-alex-component="config" data-alex-part="pageHeaderInfo">
            <PageHeader
              level={2}
              size="md"
              title={(
                <span
                  className="alex-config-page-header__title"
                  data-alex-component="config"
                  data-alex-part="pageHeaderTitle"
                >
                  {title}
                </span>
              )}
              description={subtitle ? (
                <span className="alex-config-page-header__subtitle" data-alex-component="config" data-alex-part="pageHeaderSubtitle">{formatStandaloneUiText(subtitle)}</span>
              ) : undefined}
            />
          </div>
        </div>
        {extra && (
          <div className="alex-config-page-header__extra" data-alex-component="config" data-alex-part="pageHeaderExtra">
            {extra}
          </div>
        )}
      </div>
    </div>
  );
};

export default ConfigPageHeader;
