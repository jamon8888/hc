import { Alert, Button, DialogBody, DialogClose, DialogFooter, DialogHeader, DialogHeading, DialogTitle } from '@alex/ui';
import ReactMarkdown from 'react-markdown';
import { useI18n } from '@/infrastructure/i18n';
import { systemAPI } from '@/infrastructure/api/service-api/SystemAPI';
import { getVersionInfo } from '@/shared/utils/version';
import { useUpdateInstallStore } from './updateInstallStore';
import { formatUpdateInstallError, isUpdateVersionChangedError } from './updateErrorMessage';
import { getUpdateDownloadFraction } from './updateDownloadProgress';
import './AppUpdatePanel.scss';
import './UpdateInstallProgressModal.scss';

/** Release reading and update actions share the standard dialog anatomy. */
export function AppUpdatePanel() {
  const { t } = useI18n('common');
  const state = useUpdateInstallStore();
  const downloading = state.status === 'downloading';
  const candidate = state.availableUpdate?.latestVersion;
  const target = (downloading ? state.downloadVersion : null) ?? candidate ?? state.version;
  const releaseNotes = target === candidate ? state.availableUpdate?.releaseNotes?.trim() : undefined;
  const skipped = Boolean(target && target === state.skippedVersion);
  const busy = !state.initialized || state.checkStatus === 'checking' || downloading || state.status === 'installing';
  const versionChanged = isUpdateVersionChangedError(state.error);
  const canDownload = !versionChanged && Boolean((candidate && candidate !== state.version) || state.error);
  const canSkip = Boolean(target && target !== state.version);
  const fraction = getUpdateDownloadFraction(state.progress);
  const percent = fraction == null ? null : Math.round(fraction * 100);
  const productName = getVersionInfo().name;
  const status = state.checkStatus === 'checking' ? t('update.checking')
    : state.checkStatus === 'latest' && !candidate && !state.version ? t('update.noUpdate') : null;

  return (
    // Preserve the published Appearance ancestry for existing update skins.
    <section className="alex-update-details" data-testid="app-update-panel"
      data-alex-component="about-dialog" data-alex-part="updateCard"
      data-alex-state={state.checkStatus === 'checking' ? 'checking' : downloading ? 'downloading'
        : state.error || state.checkError ? 'error' : state.status === 'ready' || state.status === 'installing' ? 'installed'
          : state.checkStatus === 'latest' ? 'latest' : undefined} aria-label={t('update.thisDevice')}>
      <DialogHeader className="alex-update-details__header">
        <DialogHeading>
          <DialogTitle>
            <span data-alex-component="update" data-alex-part="notesLabel">
              {target ? <span className="alex-update-details__versions"
                data-alex-component="update" data-alex-part="versions">
                <span data-alex-component="update" data-alex-part="versionRow">
                  <span data-alex-component="update" data-alex-part="versionLabel">{productName}</span>
                  <span className="alex-update-details__version"
                    data-alex-component="update" data-alex-part="versionValue">v{target}</span>
                </span>
              </span> : t('update.detailsTitle')}
            </span>
          </DialogTitle>
        </DialogHeading>
        <DialogClose className="alex-update-details__close" />
      </DialogHeader>
      <DialogBody className="alex-update-details__body">
        <div className="alex-update-panel" data-alex-component="update" data-alex-part="panel">
          {(status || state.error || state.checkError) && <div className="alex-update-panel__feedback"
            data-alex-component="about-dialog" data-alex-part="updateFeedback">
            {status && <p className="alex-update-panel__status" role="status"
              data-alex-component="update" data-alex-part="panelStatus">{status}</p>}
            {state.error && <Alert tone="error" showIcon message={formatUpdateInstallError(state.error, t)} />}
            {state.checkError && <Alert tone="error" showIcon message={formatUpdateInstallError(state.checkError, t)} />}
          </div>}
          {releaseNotes ? (
            <section className="alex-update-panel__notes" aria-label={t('update.releaseNotes')}
              data-alex-component="update" data-alex-part="notes">
              <div className="alex-update-panel__prose" data-alex-component="update" data-alex-part="notesBody">
                <ReactMarkdown skipHtml components={{
                  img: () => null,
                  ul: ({ children }) => <ul role="list">{children}</ul>,
                  a: ({ href, children }) => <a href={href} onClick={event => {
                    event.preventDefault();
                    if (href && /^https?:\/\//i.test(href)) void systemAPI.openExternal(href);
                  }}>{children}</a>,
                }}>{releaseNotes}</ReactMarkdown>
              </div>
            </section>
          ) : (
            <div className="alex-update-panel__empty" aria-hidden="true"
              data-alex-component="update" data-alex-part="releaseArtwork">
              <span className="alex-update-details__mark" />
            </div>
          )}
        </div>
      </DialogBody>
      <DialogFooter separator className="alex-update-details__footer">
        <div className="alex-update-panel__actions" data-alex-component="about-dialog" data-alex-part="updateActions">
          {downloading ? (
            <div className="alex-update-progress__bar alex-update-panel__progress"
              role="progressbar" aria-label={t('update.downloadingTitle')}
              aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent ?? undefined}
              data-alex-component="about-dialog" data-alex-part="progress">
              <div className={'alex-update-progress__fill' + (fraction == null ? ' alex-update-progress__fill--indeterminate' : '')}
                style={fraction == null ? undefined : { transform: `scaleX(${fraction})` }}
                data-alex-component="about-dialog" data-alex-part="progressFill" />
            </div>
          ) : (
            <>
              {canSkip && target && <Button size="md" variant="fill" disabled={busy}
                onClick={() => skipped ? state.restoreReminder(target) : state.skipVersion(target)}>
                {t(skipped ? 'update.restoreReminder' : 'update.skipVersion')}
              </Button>}
              <div className="alex-update-panel__primary-actions">
                {canDownload && <Button size="md" variant={state.version ? 'outline' : 'primary'}
                  disabled={busy} onClick={() => void state.startInstall(Boolean(state.version), target ?? undefined)}>
                  {t(state.error ? 'update.retryDownload' : 'update.downloadUpdate')}
                </Button>}
                {versionChanged && <Button size="md" variant="primary" disabled={busy} loading={state.checkStatus === 'checking'}
                  onClick={() => void state.checkForUpdates()}>
                  {t(state.checkStatus === 'checking' ? 'update.checking' : 'update.checkForUpdates')}
                </Button>}
                {state.version && <Button size="md" variant="primary" disabled={busy} onClick={state.requestInstall}>
                  {t(state.status === 'installing' ? 'update.installing' : 'update.installAndRestart')}
                  {candidate && candidate !== state.version && ` · v${state.version}`}
                </Button>}
                {!canDownload && !canSkip && !versionChanged && !state.version && <Button size="md" variant="fill" onClick={state.closeDetails}>
                  {t('actions.close')}
                </Button>}
              </div>
            </>
          )}
        </div>
      </DialogFooter>
    </section>
  );
}
