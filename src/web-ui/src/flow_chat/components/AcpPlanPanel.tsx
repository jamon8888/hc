import { Icon } from '@alex/ui';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { CircleDashed, LoaderCircle } from 'lucide-react';

import type { AcpPlanEntry } from '@/infrastructure/api/service-api/ACPClientAPI';
import './AcpPlanPanel.scss';

export interface AcpPlanPanelProps {
  entries: AcpPlanEntry[];
}

function statusIcon(status: string): React.ReactNode {
  switch (status) {
    case 'completed':
      return <Icon name="check-line" size="lg" style={{ width: 13, height: 13 }} className="alex-acp-plan__icon alex-acp-plan__icon--done" />;
    case 'in_progress':
      return (
        <LoaderCircle
          size={13}
          className="alex-acp-plan__icon alex-acp-plan__icon--active"
        />
      );
    default:
      return (
        <CircleDashed size={13} className="alex-acp-plan__icon alex-acp-plan__icon--pending" />
      );
  }
}

export const AcpPlanPanel: React.FC<AcpPlanPanelProps> = ({ entries }) => {
  const { t } = useTranslation('flow-chat');
  if (entries.length === 0) return null;

  const done = entries.filter((entry) => entry.status === 'completed').length;

  return (
    <div data-alex-component="acp-plan-panel" data-alex-part="root" className="alex-acp-plan" data-testid="acp-plan-panel">
      <div data-alex-component="acp-plan-panel" data-alex-part="header" className="alex-acp-plan__header">
        <span data-alex-component="acp-plan-panel" data-alex-part="title" className="alex-acp-plan__title">{t('chatInput.acpPlan.title')}</span>
        <span data-alex-component="acp-plan-panel" data-alex-part="progress" className="alex-acp-plan__progress">
          {done}/{entries.length}
        </span>
      </div>
      <ul data-alex-component="acp-plan-panel" data-alex-part="list" className="alex-acp-plan__list">
        {entries.map((entry, index) => (
          <li
            key={`${index}-${entry.content}`}
            data-alex-component="acp-plan-panel"
            data-alex-part="item"
            data-alex-status={entry.status}
            className={`alex-acp-plan__item alex-acp-plan__item--${entry.status}`}
          >
            {statusIcon(entry.status)}
            <span data-alex-component="acp-plan-panel" data-alex-part="content" className="alex-acp-plan__content">{entry.content}</span>
          </li>
        ))}
      </ul>
    </div>
  );
};

AcpPlanPanel.displayName = 'AcpPlanPanel';
