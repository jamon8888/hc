import React, { useId, useState } from 'react';
import { Disclosure, OverflowText, Icon, IconButton } from '@alex/ui';
import './ConfigCollectionItem.scss';

export interface ConfigCollectionItemProps extends React.HTMLAttributes<HTMLDivElement> {
  label: React.ReactNode;
  badge?: React.ReactNode;
  badgePlacement?: 'inline' | 'below';
  control: React.ReactNode;
  details?: React.ReactNode;
  disabled?: boolean;
  /** Separates details interaction from the item's disabled appearance. */
  detailsDisabled?: boolean;
  expanded?: boolean;
  onToggle?: () => void;
  /** Lets non-interactive row space toggle details while preserving nested controls. */
  toggleOnRowClick?: boolean;
  className?: string;
}

export const ConfigCollectionItem: React.FC<ConfigCollectionItemProps> = ({
  label,
  badge,
  badgePlacement = 'inline',
  control,
  details,
  disabled = false,
  detailsDisabled = disabled,
  expanded: expandedProp,
  onToggle,
  toggleOnRowClick = false,
  className = '',
  ...rootProps
}) => {
  const [internalExpanded, setInternalExpanded] = useState(false);
  const isControlled = expandedProp !== undefined;
  const isExpanded = isControlled ? expandedProp : internalExpanded;
  const hasDetails = Boolean(details);
  const labelId = useId();

  const toggleDetails = () => {
    if (!hasDetails || detailsDisabled) return;
    if (isControlled) {
      onToggle?.();
    } else {
      setInternalExpanded((prev) => !prev);
    }
  };

  const handleRowClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!toggleOnRowClick || !hasDetails || detailsDisabled) return;
    const target = event.target;
    if (
      target instanceof Element
      && target.closest('a, button, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [contenteditable="true"]')
    ) {
      return;
    }
    toggleDetails();
  };

  return (
    <div
      className={`alex-collection-item ${isExpanded ? 'is-expanded' : ''} ${disabled ? 'is-disabled' : ''} ${className}`}
      data-alex-component="config"
      data-alex-part="collectionItem"
      {...rootProps}
    >
      <Disclosure
        className="alex-collection-item__disclosure"
        summary={label}
        open={hasDetails && isExpanded}
        disabled={!hasDetails || detailsDisabled}
        onOpenChange={toggleDetails}
        unmountOnClose
        exitDurationMs={180}
        contentClassName="alex-collection-item__details-collapse"
        contentInnerClassName="alex-collection-item__details-clip"
        renderHeader={(triggerProps) => (
          <div data-overflow-trigger
            className={`alex-config-page-row alex-config-page-row--center alex-collection-item__row ${
              toggleOnRowClick && hasDetails && !detailsDisabled ? 'alex-collection-item__row--toggleable' : ''
            }`}
            data-alex-component="config"
            data-alex-part="collectionRow"
            onClick={handleRowClick}
          >
            <div className="alex-config-page-row__meta" data-alex-component="config" data-alex-part="collectionMeta">
              <div
                className={`alex-config-page-row__label alex-collection-item__label ${
                  badgePlacement === 'below' ? 'alex-collection-item__label--stacked' : ''
                }`}
              >
                <OverflowText id={labelId} className="alex-collection-item__name" data-alex-component="config" data-alex-part="collectionName">{label}</OverflowText>
                {badge && (
                  <span
                    className={`alex-collection-item__badges ${
                      badgePlacement === 'below'
                        ? 'alex-collection-item__badges--stacked'
                        : 'alex-collection-item__badges--inline'
                    }`}
                  >
                    {badge}
                  </span>
                )}
              </div>
            </div>
            <div className="alex-config-page-row__control" data-alex-component="config" data-alex-part="collectionControl">
              <div className="alex-collection-item__control">
                {control}
                {hasDetails ? (
                  <IconButton
                    type="button"
                    className="alex-collection-btn alex-collection-item__details-toggle"
                    disabled={detailsDisabled}
                    aria-label={typeof label === 'string' ? label : ''}
                    aria-labelledby={labelId}
                    {...triggerProps}
                    icon={<Icon name="chevron-down" size="sm" aria-hidden="true" />}
                  />
                ) : null}
              </div>
            </div>
          </div>
        )}
      >
        <div className="alex-collection-item__details" data-alex-component="config" data-alex-part="collectionDetails">{details}</div>
      </Disclosure>
    </div>
  );
};

export default ConfigCollectionItem;
