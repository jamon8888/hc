/**
 * About dialog component.
 * Shows product identity, build metadata, license, and the
 * persistent GitHub repository entry point.
 */

import {
  Button,
  FieldGroup,
  FieldRow,
  Icon,
  IconButton,
  StatusPill,
  Tooltip,
  Dialog,
  DialogBody,
  DialogClose,
  DialogTitle,
} from '@alex/ui';
import React, { useCallback, useEffect, useState } from 'react';
import { useI18n } from '@/infrastructure/i18n';
import {
  formatBuildDate,
  formatDisplayedVersion,
  getAboutInfo,
} from '@/shared/utils/version';
import { createLogger } from '@/shared/utils/logger';
import { systemAPI } from '@/infrastructure/api';
import { isTauriRuntime } from '@/infrastructure/update/tauriEnv';
import { AboutBrandMark } from './AboutBrandMark';
import './AboutDialog.scss';

const log = createLogger('AboutDialog');
const GITHUB_REPOSITORY_URL = 'https://github.com/GCWing/OpenBitFun';

interface AboutDialogProps {
  /** Whether visible */
  isOpen: boolean;
  /** Close callback */
  onClose: () => void;
}

export const AboutDialog: React.FC<AboutDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const { t } = useI18n('common');
  const [copiedItem, setCopiedItem] = useState<string | null>(null);
  const [nativeVersion, setNativeVersion] = useState<string | null>(null);

  const aboutInfo = getAboutInfo();
  const { version, license } = aboutInfo;
  const nativeRuntime = isTauriRuntime();
  const displayedVersion = formatDisplayedVersion(
    version,
    nativeVersion,
    nativeRuntime,
    import.meta.env.DEV,
  );
  const licenseName = license.type === 'MIT' ? 'MIT License' : license.type;
  const licenseCopyright = license.text?.startsWith(`${licenseName} - `)
    ? license.text.slice(`${licenseName} - `.length)
    : license.text;
  const legalCopyright = licenseCopyright
    ? `${licenseCopyright.replace(/\.$/, '')}. ${t('about.allRightsReserved')}`
    : t('about.copyright');

  let releaseLabel = t('about.stableBuild');
  if (displayedVersion.endsWith('-dev')) {
    releaseLabel = t('about.developmentBuild');
  } else if (version.releaseChannel === 'beta') {
    releaseLabel = t('about.betaBuild');
  } else if (version.releaseChannel === 'nightly') {
    releaseLabel = t('about.nightlyBuild');
  }

  useEffect(() => {
    if (!isOpen || !nativeRuntime) return;
    let active = true;
    void systemAPI.getLocalAppVersion()
      .then(currentVersion => {
        if (active) setNativeVersion(currentVersion);
      })
      .catch(error => {
        log.warn('Failed to read the local version; using generated version metadata', error);
      });
    return () => {
      active = false;
    };
  }, [isOpen, nativeRuntime]);

  const handleGithubStar = useCallback(() => {
    systemAPI.openExternal(GITHUB_REPOSITORY_URL).catch(error => {
      log.error('Failed to open the GitHub repository', { url: GITHUB_REPOSITORY_URL, error });
    });
  }, []);

  const copyToClipboard = async (text: string, itemId: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedItem(itemId);
      window.setTimeout(() => setCopiedItem(null), 2000);
    } catch (error) {
      log.error('Failed to copy to clipboard', error);
    }
  };

  return (
    <>
      <Dialog
        open={isOpen}
        onOpenChange={(nextOpen) => { if (!nextOpen) onClose(); }}
        size="xl"
        className="alex-about-dialog"
        aria-label={t('about.dialogTitle')}
        data-testid="about-dialog-modal"
      >
        <DialogClose className="alex-about-dialog__close" />
        <DialogBody className="alex-about-dialog__modal-content" inset="none">
          <div
            className="alex-about-dialog__content"
            data-alex-component="about-dialog"
            data-alex-part="root"
          >
            <div className="alex-about-dialog__body">
              <div
                className="alex-about-dialog__brand"
                data-alex-component="about-dialog"
                data-alex-part="hero"
                aria-hidden="true"
              >
                <div className="alex-about-dialog__artwork">
                  <AboutBrandMark active={isOpen} />
                </div>
                <p className="alex-about-dialog__brand-statement">
                  {t('about.brandStatement')}
                </p>
              </div>

              <section
                className="alex-about-dialog__metadata"
                data-alex-component="about-dialog"
                data-alex-part="content"
                aria-label={t('about.details')}
              >
                <header className="alex-about-dialog__brand-copy">
                  <DialogTitle
                    className="alex-about-dialog__title"
                    data-alex-component="about-dialog"
                    data-alex-part="title"
                  >
                    {version.name}
                  </DialogTitle>
                  <p className="alex-about-dialog__tagline">{t('about.tagline')}</p>
                </header>

                <FieldGroup className="alex-about-dialog__details" appearance="plain" dividers={false}>
                  <FieldRow padding="none">
                    <dl className="alex-about-dialog__info-row" data-alex-component="about-dialog" data-alex-part="infoRow">
                      <dt className="alex-about-dialog__info-label" data-alex-component="about-dialog" data-alex-part="infoLabel">
                        <span>{t('about.versionLabel')}</span>
                      </dt>
                      <dd className="alex-about-dialog__info-value-group">
                        <span
                          className="alex-about-dialog__info-value"
                          data-alex-component="about-dialog"
                          data-alex-part="infoValue"
                          data-testid="about-version-value"
                        >
                          {displayedVersion}
                        </span>
                        <span
                          className="alex-about-dialog__channel-badge"
                          data-alex-component="about-dialog"
                          data-alex-part="channelBadge"
                        >
                          <StatusPill tone="neutral">{releaseLabel}</StatusPill>
                        </span>
                      </dd>
                    </dl>
                  </FieldRow>

                  <FieldRow padding="none">
                    <dl className="alex-about-dialog__info-row" data-alex-component="about-dialog" data-alex-part="infoRow">
                      <dt className="alex-about-dialog__info-label" data-alex-component="about-dialog" data-alex-part="infoLabel">
                        <span>{t('about.buildDate')}</span>
                      </dt>
                      <dd className="alex-about-dialog__info-value-group">
                        <span className="alex-about-dialog__info-value" data-alex-component="about-dialog" data-alex-part="infoValue">
                          {formatBuildDate(version.buildDate)}
                        </span>
                      </dd>
                    </dl>
                  </FieldRow>

                  <FieldRow padding="none">
                    <dl className="alex-about-dialog__info-row" data-alex-component="about-dialog" data-alex-part="infoRow">
                      <dt className="alex-about-dialog__info-label" data-alex-component="about-dialog" data-alex-part="infoLabel">
                        <span>{t('about.commit')}</span>
                      </dt>
                      <dd className="alex-about-dialog__info-value-group">
                        <span
                          className="alex-about-dialog__info-value alex-about-dialog__info-value--mono"
                          data-alex-component="about-dialog"
                          data-alex-part="infoValue"
                        >
                          {version.gitCommit ?? t('about.notAvailable')}
                        </span>
                        {version.gitCommit ? (
                          <span
                            className="alex-about-dialog__copy-action"
                            data-alex-component="about-dialog"
                            data-alex-part="copyButton"
                          >
                            <Tooltip content={t('about.copy')}>
                              <IconButton
                                size="xs"
                                variant="quiet"
                                icon={<Icon name={copiedItem === 'commit' ? 'check-line' : 'duplicate'} size="sm" />}
                                onClick={() => void copyToClipboard(version.gitCommit ?? '', 'commit')}
                                aria-label={t('about.copyCommit')}
                              />
                            </Tooltip>
                          </span>
                        ) : null}
                      </dd>
                    </dl>
                  </FieldRow>

                  <FieldRow padding="none">
                    <dl className="alex-about-dialog__info-row" data-alex-component="about-dialog" data-alex-part="infoRow">
                      <dt className="alex-about-dialog__info-label" data-alex-component="about-dialog" data-alex-part="infoLabel">
                        <span>{t('about.branch')}</span>
                      </dt>
                      <dd className="alex-about-dialog__info-value-group">
                        <span
                          className="alex-about-dialog__info-value"
                          data-alex-component="about-dialog"
                          data-alex-part="infoValue"
                          data-testid="about-branch-value"
                          title={version.gitBranch}
                        >
                          {version.gitBranch ?? t('about.notAvailable')}
                        </span>
                      </dd>
                    </dl>
                  </FieldRow>

                  <FieldRow padding="none">
                    <dl className="alex-about-dialog__info-row" data-alex-component="about-dialog" data-alex-part="infoRow">
                      <dt className="alex-about-dialog__info-label" data-alex-component="about-dialog" data-alex-part="infoLabel">
                        <span>{t('about.license')}</span>
                      </dt>
                      <dd className="alex-about-dialog__info-value-group">
                        <span
                          className="alex-about-dialog__info-value"
                          data-alex-component="about-dialog"
                          data-alex-part="license"
                          data-testid="about-license-value"
                        >
                          {licenseName}
                        </span>
                      </dd>
                    </dl>
                  </FieldRow>
                </FieldGroup>

              </section>
            </div>

            <footer
              className="alex-about-dialog__footer"
              data-alex-component="about-dialog"
              data-alex-part="footer"
            >
              <div
                className="alex-about-dialog__star-callout"
                data-alex-component="about-dialog"
                data-alex-part="starCallout"
                role="group"
                aria-labelledby="alex-about-star-title"
              >
                <div className="alex-about-dialog__star-copy">
                  <h3 id="alex-about-star-title" className="alex-about-dialog__star-title">
                    {t('about.githubStarTitle')}
                  </h3>
                  <p className="alex-about-dialog__star-description">
                    {t('about.githubStarDescription')}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="alex-about-dialog__star-button"
                  leadingIcon={<Icon name="star" size="sm" aria-hidden="true" />}
                  onClick={handleGithubStar}
                  data-testid="about-github-star"
                >
                  {t('about.githubStarAction')}
                </Button>
              </div>
              <p
                className="alex-about-dialog__copyright"
                data-alex-component="about-dialog"
                data-alex-part="copyright"
              >
                {legalCopyright}
              </p>
            </footer>
          </div>
        </DialogBody>
      </Dialog>

    </>
  );
};

export default AboutDialog;
