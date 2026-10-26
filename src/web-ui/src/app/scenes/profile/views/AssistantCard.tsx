import React from 'react';
import {
  Button,
  Card,
  CardFooter,
  CardHeader,
  Icon,
  IconButton,
  OverflowText,
  StatusPill,
  Tooltip,
} from '@alex/ui';

import { useTranslation } from 'react-i18next';

import { AssistantAvatar } from '@/app/components/AssistantAvatar';
import type { WorkspaceInfo } from '@/shared/types';

interface AssistantCardProps {
  workspace: WorkspaceInfo;
  onClick: () => void;
  onNewSession?: () => void;
  onDelete?: () => void;
  onSetPrimary?: () => void;
  isPrimary?: boolean;
  isDeleting?: boolean;
  isStartingSession?: boolean;
  isSettingPrimary?: boolean;
  style?: React.CSSProperties;
}

const AssistantCard: React.FC<AssistantCardProps> = ({
  workspace,
  onClick,
  onNewSession,
  onDelete,
  onSetPrimary,
  isPrimary,
  isDeleting = false,
  isStartingSession = false,
  isSettingPrimary = false,
  style,
}) => {
  const { t } = useTranslation('scenes/profile');
  const identity = workspace.identity;

  const name = identity?.name?.trim() || workspace.name || t('nursery.card.unnamed');
  const avatar = identity?.avatar?.trim() ?? '';
  const emoji = identity?.emoji?.trim() ?? '';
  const creature = identity?.creature?.trim() || '';
  const vibe = identity?.vibe?.trim() || '';
  const isBusy = isDeleting || isStartingSession || isSettingPrimary;
  const hasActions = Boolean(onNewSession || onSetPrimary || onDelete);

  return (
    <article
      data-alex-component="assistant-card"
      data-alex-part="root"
      data-alex-primary={isPrimary ? 'true' : 'false'}
      data-alex-state={isBusy ? 'busy' : undefined}
      className={['assistant-card', (isDeleting || isSettingPrimary) && 'assistant-card--busy'].filter(Boolean).join(' ')}
      role="listitem"
      style={style}
    >
      <Card
        className="assistant-card__surface"
        appearance="subtle"
        radius="md"
        padding="md"
        gap="sm"
        clip
        data-overflow-trigger
      >
        <button
          data-alex-component="assistant-card"
          data-alex-part="main"
          type="button"
          className="assistant-card__main"
          onClick={onClick}
          aria-label={`${t('nursery.card.configure')}: ${name}`}
          disabled={isDeleting || isSettingPrimary}
        />

        <CardHeader
          align="center"
          className="assistant-card__header"
          data-alex-component="assistant-card"
          data-alex-part="header"
          leading={(
            <span className="assistant-card__avatar" data-alex-component="assistant-card" data-alex-part="avatar">
              <AssistantAvatar
                presetId={avatar}
                emoji={emoji}
                stableKey={workspace.assistantId || workspace.id}
                name={name}
                size={44}
              />
            </span>
          )}
          title={(
            <span className="assistant-card__title-row" data-alex-component="assistant-card" data-alex-part="title">
              <OverflowText className="assistant-card__name" data-alex-component="assistant-card" data-alex-part="name">{name}</OverflowText>
              {isPrimary && (
                <span className="assistant-card__primary-badge" data-alex-component="assistant-card" data-alex-part="primaryBadge">
                  <StatusPill tone="neutral">{t('nursery.card.primaryBadge')}</StatusPill>
                </span>
              )}
            </span>
          )}
          description={(
            <span className="assistant-card__metadata" data-alex-component="assistant-card" data-alex-part="metadata">
              {vibe ? (
                <OverflowText className="assistant-card__vibe" data-alex-component="assistant-card" data-alex-part="vibe">{vibe}</OverflowText>
              ) : (
                <OverflowText lines={1} className="assistant-card__vibe assistant-card__vibe--empty" data-alex-component="assistant-card" data-alex-part="vibe">
                  {t('nursery.card.noVibe')}
                </OverflowText>
              )}
              {creature ? (
                <>
                  <span className="assistant-card__metadata-separator" aria-hidden="true">·</span>
                  <OverflowText className="assistant-card__creature" data-alex-component="assistant-card" data-alex-part="creature">
                    {creature}
                  </OverflowText>
                </>
              ) : null}
            </span>
          )}
          actions={(
            <Icon
              name="chevron-right"
              size="sm"
              data-alex-component="assistant-card"
              data-alex-part="chevron"
              className="assistant-card__chevron"
              aria-hidden="true"
            />
          )}
        />

        {hasActions ? (
          <CardFooter align="end" className="assistant-card__footer" data-alex-component="assistant-card" data-alex-part="footer">
            <span className="assistant-card__session-actions">
              {onNewSession ? (
                <Button
                  variant="primary"
                  size="sm"
                  leadingIcon={<Icon name="side-chat" size="sm" />}
                  loading={isStartingSession}
                  onClick={onNewSession}
                  disabled={isStartingSession || isDeleting || isSettingPrimary}
                >
                  {t(isStartingSession ? 'nursery.card.startingSession' : 'nursery.card.newSession')}
                </Button>
              ) : null}

              <span className="assistant-card__footer-actions">
                {onSetPrimary ? (
                  <Tooltip content={t('nursery.card.setPrimary')}>
                    <IconButton
                      data-alex-component="assistant-card"
                      data-alex-part="setPrimary"
                      size="sm"
                      onClick={onSetPrimary}
                      aria-label={t('nursery.card.setPrimary')}
                      loading={isSettingPrimary}
                      disabled={isDeleting || isStartingSession || isSettingPrimary}
                      icon={<Icon name="pin" size="sm" aria-hidden="true" />}
                    />
                  </Tooltip>
                ) : null}

                {onDelete ? (
                  <Tooltip content={t('nursery.card.delete')}>
                    <IconButton
                      data-alex-component="assistant-card"
                      data-alex-part="delete"
                      tone="danger"
                      size="sm"
                      onClick={onDelete}
                      aria-label={t('nursery.card.delete')}
                      loading={isDeleting}
                      disabled={isDeleting || isStartingSession || isSettingPrimary}
                      icon={<Icon name="delete" size="sm" aria-hidden="true" />}
                    />
                  </Tooltip>
                ) : null}
              </span>
            </span>
          </CardFooter>
        ) : null}
      </Card>
    </article>
  );
};

export default AssistantCard;
