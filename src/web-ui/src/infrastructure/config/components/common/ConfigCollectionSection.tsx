import React from 'react';
import { ConfigPageSection } from './ConfigPageLayout';
import './ConfigCollectionSection.scss';

export interface ConfigCollectionSectionProps {
  title: string;
  description?: string;
  toolbar?: React.ReactNode;
  filters?: React.ReactNode;
  editor?: React.ReactNode;
  className?: string;
  children: React.ReactNode;
}

export const ConfigCollectionSection: React.FC<ConfigCollectionSectionProps> = ({
  title,
  description,
  toolbar,
  filters,
  editor,
  className = '',
  children,
}) => {
  const hasEditor = Boolean(editor);

  return (
    <ConfigPageSection
      title={title}
      description={description}
      className={`alex-config-collection-section ${hasEditor ? 'alex-config-collection-section--with-editor' : ''} ${className}`}
    >
      <div className="alex-config-collection-section__content" data-alex-component="config" data-alex-part="collectionSection">
        {toolbar && (
          <div className="alex-config-collection-section__toolbar" data-alex-component="config" data-alex-part="collectionToolbar">
            {toolbar}
          </div>
        )}
        {editor && (
          <div className="alex-config-collection-section__editor" data-alex-component="config" data-alex-part="collectionEditor">
            {editor}
          </div>
        )}
        {filters && (
          <div className="alex-config-collection-section__filters" data-alex-component="config" data-alex-part="collectionFilters">
            {filters}
          </div>
        )}
        <div className="alex-config-collection-section__list" data-alex-component="config" data-alex-part="collectionList">
          {children}
        </div>
      </div>
    </ConfigPageSection>
  );
};

export default ConfigCollectionSection;
