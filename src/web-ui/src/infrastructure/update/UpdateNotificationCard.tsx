import { ActionItem, Button, Card, CardBody, CardFooter, CardHeader, Icon, IconButton, OverlayLayer, OverflowText, useHasModalOverlay } from '@alex/ui';
import { useEffect, useMemo, useRef, type MouseEvent } from 'react';
import { useI18n } from '@/infrastructure/i18n';
import { getVersionInfo } from '@/shared/utils/version';
import { canCheckForAppUpdates } from './tauriEnv';
import { useUpdateInstallStore, type UpdateNotice } from './updateInstallStore';
import { formatUpdateInstallError, isUpdateVersionChangedError } from './updateErrorMessage';
import { getUpdateIntroduction } from './updateReleaseNotes';
import { prepareUpdateDownloadHandoff } from './updateDownloadMotion';
import './UpdateInstallProgressModal.scss';
import './UpdateNotificationCard.scss';

function UpdateNoticeCard({ notice }: { notice: UpdateNotice }) {
  const { t } = useI18n('common');
  const state = useUpdateInstallStore();
  const { noticeRevision, markNoticePresented } = state;
  const modalOpen = useHasModalOverlay();
  const version = notice === 'available' ? state.availableUpdate?.latestVersion
    : state.downloadVersion ?? state.availableUpdate?.latestVersion ?? state.version;
  const release = version && version === state.availableUpdate?.latestVersion ? state.availableUpdate : null;
  const releaseNotes = release?.releaseNotes;
  const introduction = useMemo(() => getUpdateIntroduction(releaseNotes), [releaseNotes]);
  const cardRef = useRef<HTMLDivElement>(null);
  const cancelHandoff = useRef<(() => void) | undefined>();
  useEffect(() => () => cancelHandoff.current?.(), []);

  const startDownload = (event: MouseEvent<HTMLButtonElement>, replacePending: boolean, target: string | undefined) => {
    cancelHandoff.current?.();
    const cancel = cardRef.current && target
      ? prepareUpdateDownloadHandoff(event.currentTarget, cardRef.current, target, event.detail === 0) : undefined;
    cancelHandoff.current = cancel;
    void state.startInstall(replacePending, target).finally(() => cancel?.());
  };

  useEffect(() => {
    const markPresented = () => {
      if (!modalOpen && document.visibilityState !== 'hidden' && document.hasFocus()) markNoticePresented();
    };
    markPresented();
    window.addEventListener('focus', markPresented);
    document.addEventListener('visibilitychange', markPresented);
    return () => {
      window.removeEventListener('focus', markPresented);
      document.removeEventListener('visibilitychange', markPresented);
    };
  }, [modalOpen, notice, version, noticeRevision, markNoticePresented]);

  const percent = state.progress.total && state.progress.total > 0
    ? Math.min(100, Math.round(state.progress.downloaded / state.progress.total * 100)) : null;
  const title = version ? t('update.releaseTitle', { name: getVersionInfo().name, version })
    : t('update.availableTitle');
  const message = notice === 'error' ? formatUpdateInstallError(state.error ?? '', t) : null;

  return (
    <div
      ref={cardRef}
      className="alex-update-notice"
      data-testid="app-update-notice"
      data-alex-component="update"
      data-alex-part="notice"
      onKeyDown={event => {
        if (event.key === 'Escape') {
          event.stopPropagation();
          state.dismissNotice();
        }
      }}
    >
      <Card appearance="raised" padding="md" gap="md" radius="md"
        className="alex-update-notice__surface" data-alex-native-webview-occlusion>
        <div data-alex-component="update" data-alex-part="availableRoot" data-alex-variant="daily">
          <div data-alex-component="update" data-alex-part="lead">
            <CardHeader
              align="center"
              actions={(
                <IconButton icon={<Icon name="xmark" size="sm" />} size="xs" variant="quiet"
                  aria-label={t('actions.close')} onClick={state.dismissNotice} />
              )}
            >
              <span className="alex-update-notice__heading"
                data-alex-component="update" data-alex-part="noticeTitle">
                <ActionItem className="alex-update-notice__title"
                  aria-label={t('update.openReleaseNotes', { title })}
                  aria-haspopup="dialog" data-testid="app-update-release-notes"
                  metadata={<Icon name="chevron-right" size="xs" />}
                  onClick={state.openReleaseNotes}>
                  <OverflowText className="alex-update-notice__title-text">{title}</OverflowText>
                </ActionItem>
              </span>
            </CardHeader>
          </div>
        </div>
        <CardBody className="alex-update-notice__body">
          {introduction ? (
            <OverflowText as="p" lines={notice === 'error' ? 1 : 3} className="alex-update-notice__introduction"
              data-testid="app-update-introduction"
              data-alex-component="update" data-alex-part="subtitle">
              {introduction}
            </OverflowText>
          ) : (
            <div className="alex-update-notice__empty-introduction" aria-hidden="true"
              data-testid="app-update-release-artwork"
              data-alex-component="update" data-alex-part="releaseArtwork">
              <span className="alex-update-notice__brand-mark" />
            </div>
          )}
          {notice === 'error' && <div className="alex-update-notice__feedback">
            <p className="alex-update-notice__message" role="status" aria-live="polite"
              data-alex-component="update" data-alex-part="subtitle">
              <span className="alex-update-notice__status-icon" aria-hidden="true"
                data-alex-component="update" data-alex-part="leadIcon">
                <Icon name="info" size="sm" tone="danger" />
              </span>
              <OverflowText lines={2}>{message}</OverflowText>
            </p>
          </div>}
          {notice === 'downloading' && (
            <div className="alex-update-progress__bar alex-update-notice__progress"
              role="progressbar" aria-label={t('update.downloadingTitle')}
              aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent ?? undefined}
              data-alex-component="update" data-alex-part="progressBar">
              <div
                className={'alex-update-progress__fill' + (percent == null ? ' alex-update-progress__fill--indeterminate' : '')}
                style={percent == null ? undefined : { transform: `scaleX(${percent / 100})` }}
                data-alex-component="update" data-alex-part="progressFill"
                data-alex-state={percent == null ? 'indeterminate' : undefined} />
            </div>
          )}
        </CardBody>
        {notice !== 'downloading' && (
          <div className="alex-update-notice__footer" data-alex-component="update" data-alex-part="actions">
            <CardFooter align={notice === 'available' ? 'between' : 'end'} className="alex-update-notice__actions">
              {notice === 'available' && version && (
                <Button className="alex-update-notice__action" size="sm" variant="outline" onClick={() => state.skipVersion(version)}>
                  {t('update.skipVersion')}
                </Button>
              )}
              {notice === 'available' && <Button className="alex-update-notice__action" size="sm" variant="primary" onClick={event => startDownload(event, Boolean(state.version), version ?? undefined)}>
                {t('update.downloadUpdate')}
              </Button>}
              {notice === 'error' && <Button className="alex-update-notice__action" size="sm" variant="primary" onClick={event => {
                if (isUpdateVersionChangedError(state.error)) {
                  state.openDetails();
                  void state.checkForUpdates();
                } else startDownload(event, true, state.downloadVersion ?? undefined);
              }}>
                {t(isUpdateVersionChangedError(state.error) ? 'update.checkForUpdates' : 'update.retryDownload')}
              </Button>}
            </CardFooter>
          </div>
        )}
      </Card>
    </div>
  );
}

/** Persistent until an explicit action; update attention is independent of this notice. */
export function UpdateNotificationCard() {
  const notice = useUpdateInstallStore(state => state.notice);
  const detailsOpen = useUpdateInstallStore(state => state.detailsOpen);
  if (!canCheckForAppUpdates() || !notice || detailsOpen) return null;
  return <OverlayLayer passive><UpdateNoticeCard notice={notice} /></OverlayLayer>;
}
