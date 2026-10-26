/**
 * One Todo row, shared by the 24-hour list and the calendar day detail.
 *
 * A row represents a single occurrence, so the same recurring job can appear on
 * several rows with different times.
 */

import { OverflowText, Icon, IconButton, Switch, Tooltip } from '@alex/ui';
import React from 'react';
import { CalendarClock } from 'lucide-react';

import { useI18n } from '@/infrastructure/i18n';
import type { CronJob } from '@/infrastructure/api';
import type { WorkspaceInfo } from '@/shared/types';
import {
  formatCountdown,
  formatJobTargetLabel,
  formatScheduleSummary,
  formatTimeOfDay,
  resolveJobWorkspaceLabel,
} from '../todoPresentation';

export interface TodoItemRowProps {
  job: CronJob;
  /** Occurrence time, or null for a row in the inactive group. */
  atMs: number | null;
  isOverdue?: boolean;
  isNextRun?: boolean;
  /** The scheduler is executing this run right now. */
  isRunning?: boolean;
  nowMs: number;
  workspaces: WorkspaceInfo[];
  /** Replaces the countdown, used for inactive rows ("Paused", "Done"). */
  statusLabel?: string;
  isSelected?: boolean;
  onEdit: (job: CronJob) => void;
  onDelete: (job: CronJob) => void;
  onToggleEnabled: (job: CronJob, enabled: boolean) => void;
}

const TodoItemRow: React.FC<TodoItemRowProps> = ({
  job,
  atMs,
  isOverdue = false,
  isNextRun = false,
  isRunning = false,
  nowMs,
  workspaces,
  statusLabel,
  isSelected = false,
  onEdit,
  onDelete,
  onToggleEnabled,
}) => {
  const { t, formatDate } = useI18n(['scenes/todos', 'shared']);

  const timeLabel = atMs != null ? formatTimeOfDay(atMs, formatDate) : null;
  const relativeLabel = statusLabel
    ?? (isRunning ? t('shared:statuses.running') : null)
    ?? (atMs != null ? formatCountdown(atMs, nowMs, t) : null);

  const rowState = [
    isSelected ? 'selected' : null,
    isRunning ? 'running' : null,
    isOverdue ? 'overdue' : null,
    job.enabled ? null : 'disabled',
  ].filter(Boolean).join(' ');

  return (
    <div data-overflow-trigger
      className={[
        'alex-todos__row',
        isRunning ? 'alex-todos__row--running' : '',
        isOverdue ? 'alex-todos__row--overdue' : '',
        job.enabled ? '' : 'alex-todos__row--disabled',
        isSelected ? 'alex-todos__row--selected' : '',
      ].filter(Boolean).join(' ')}
      data-alex-scene="todos"
      data-alex-part="row"
      data-alex-state={rowState || undefined}
      data-testid="todos-row"
      role="group"
      tabIndex={0}
      aria-label={`${job.name}${timeLabel ? `, ${timeLabel}` : ''}`}
      onClick={() => onEdit(job)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onEdit(job);
        }
      }}
    >
      <div className="alex-todos__row-icon" data-alex-scene="todos" data-alex-part="rowIcon">
        <CalendarClock size={19} aria-hidden="true" />
      </div>

      <div className="alex-todos__row-body" data-alex-scene="todos" data-alex-part="rowBody">
        <div className="alex-todos__row-title-line">
          <OverflowText className="alex-todos__row-name">{job.name}</OverflowText>
          {isRunning ? (
            <span
              className="alex-todos__row-badge alex-todos__row-badge--running"
              data-alex-scene="todos"
              data-alex-part="rowBadge"
            >
              {t('shared:statuses.running')}
            </span>
          ) : isNextRun ? (
            <span className="alex-todos__row-badge" data-alex-scene="todos" data-alex-part="rowBadge">
              {t('badges.nextRun')}
            </span>
          ) : null}
          {isOverdue ? (
            <span
              className="alex-todos__row-badge alex-todos__row-badge--warn"
              data-alex-scene="todos"
              data-alex-part="rowBadge"
            >
              {t('badges.overdue')}
            </span>
          ) : null}
        </div>
        <div className="alex-todos__row-meta"><OverflowText behavior="marquee">
          <span>{resolveJobWorkspaceLabel(job, workspaces)}</span>
          <span className="alex-todos__row-meta-sep" aria-hidden="true">·</span>
          <span>{formatScheduleSummary(job.schedule, t, formatDate)}</span>
          <span className="alex-todos__row-meta-sep" aria-hidden="true">·</span>
          <span>{formatJobTargetLabel(job, t)}</span>
          {relativeLabel ? (
            <>
              <span className="alex-todos__row-meta-sep" aria-hidden="true">·</span>
              <span title={timeLabel ?? undefined}>{relativeLabel}</span>
            </>
          ) : null}
        </OverflowText></div>
        {job.state.lastError ? (
          <p className="alex-todos__row-error" data-alex-scene="todos" data-alex-part="rowError">
            {job.state.lastError}
          </p>
        ) : null}
      </div>

      <div
        className="alex-todos__row-actions"
        data-alex-scene="todos"
        data-alex-part="rowActions"
        onClick={(event) => event.stopPropagation()}
        role="presentation"
      >
        <Switch
          checked={job.enabled}
          aria-label={t('actions.toggleEnabled')}
          onChange={(event) => onToggleEnabled(job, event.currentTarget.checked)}
        />
        <div className="alex-todos__row-action-buttons">
          <Tooltip content={t('actions.edit')}>
            <IconButton
              type="button"
              size="sm"
              aria-label={t('actions.edit')}
              icon={<Icon name="edit" size="lg" />}
              onClick={() => onEdit(job)}
            />
          </Tooltip>
          <Tooltip content={t('actions.delete')}>
            <IconButton
              type="button"
              size="sm"
              tone="danger"
              aria-label={t('actions.delete')}
              icon={<Icon name="delete" size="lg" />}
              onClick={() => onDelete(job)}
            />
          </Tooltip>
        </div>
      </div>
    </div>
  );
};

export default TodoItemRow;
