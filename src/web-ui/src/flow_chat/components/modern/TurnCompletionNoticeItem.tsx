import { Icon } from '@alex/ui';
import React from 'react';
import { AlertCircle, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/infrastructure/i18n';
import type { TurnCompletionNotice } from '../../utils/turnCompletionNotice';
import './TurnCompletionNoticeItem.scss';

interface TurnCompletionNoticeItemProps {
  notice: TurnCompletionNotice;
}

function getNoticeIcon(tone: TurnCompletionNotice['tone']): React.ReactNode {
  switch (tone) {
    case 'error':
      return <AlertCircle size={16} />;
    case 'info':
      return <Icon name="info" size="md" />;
    case 'warning':
    default:
      return <AlertTriangle size={16} />;
  }
}

export const TurnCompletionNoticeItem: React.FC<TurnCompletionNoticeItemProps> = ({ notice }) => {
  const { t } = useI18n('flow-chat');
  const body = notice.bodyKey ? t(notice.bodyKey) : null;

  return (
    <div data-alex-component="turn-completion-notice" data-alex-part="root" data-alex-tone={notice.tone}
      className={[
        'turn-completion-notice',
        `turn-completion-notice--${notice.tone}`,
      ].join(' ')}
      role="note"
      aria-label={t(notice.titleKey)}
    >
      <div data-alex-component="turn-completion-notice" data-alex-part="icon" className="turn-completion-notice__icon" aria-hidden="true">
        {getNoticeIcon(notice.tone)}
      </div>
      <div data-alex-component="turn-completion-notice" data-alex-part="content" className="turn-completion-notice__content">
        <div data-alex-component="turn-completion-notice" data-alex-part="text" className="turn-completion-notice__text">
          <span data-alex-component="turn-completion-notice" data-alex-part="title" className="turn-completion-notice__title">{t(notice.titleKey)}</span>
          {body ? (
            <>
              <span className="turn-completion-notice__separator" aria-hidden="true">·</span>
              <span data-alex-component="turn-completion-notice" data-alex-part="body" className="turn-completion-notice__body">{body}</span>
            </>
          ) : null}
        </div>
        <span data-alex-component="turn-completion-notice" data-alex-part="reason" className="turn-completion-notice__reason">
          <code data-alex-component="turn-completion-notice" data-alex-part="reasonCode" className="turn-completion-notice__reason-code">{notice.reasonCode}</code>
        </span>
      </div>
    </div>
  );
};
