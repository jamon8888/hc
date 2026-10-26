import {
  Button,
  DialogBody,
  DialogFooter,
  Disclosure,
  ScrollArea,
  StatusPill,
} from '@alex/ui';
import React from 'react';
import { useI18n } from '@/infrastructure/i18n';
import './RemoteConnectDisclaimer.scss';

interface RemoteConnectDisclaimerProps {
  agreed: boolean;
  onClose: () => void;
  onAgree?: () => void;
}

export const RemoteConnectDisclaimer: React.FC<RemoteConnectDisclaimerProps> = ({
  agreed,
  onClose,
  onAgree,
}) => {
  const { t } = useI18n('common');
  const canAgree = !!onAgree && !agreed;

  return (
    <>
      <DialogBody>
        <div data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="root" className="alex-remote-disclaimer">
          <div className="alex-remote-disclaimer__meta" data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="meta">
            <StatusPill tone={agreed ? 'success' : 'warning'}>
              {t(agreed ? 'remoteConnect.disclaimerStatusAgreed' : 'remoteConnect.disclaimerStatusPending')}
            </StatusPill>
          </div>

          <p className="alex-remote-disclaimer__text" data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="intro">{t('remoteConnect.disclaimerIntro')}</p>

          <h3 className="alex-remote-disclaimer__section-title" data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="title">
            {t('remoteConnect.disclaimerKeyRisks')}
          </h3>
          <ol className="alex-remote-disclaimer__list alex-remote-disclaimer__list--key" data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="riskList">
            <li>{t('remoteConnect.disclaimerItemGeneralRisk')}</li>
            <li>{t('remoteConnect.disclaimerItemSecurity')}</li>
            <li>{t('remoteConnect.disclaimerItemEncryption')}</li>
            <li>{t('remoteConnect.disclaimerItemPrivacy')}</li>
          </ol>

          <Disclosure presentation="native" className="alex-remote-disclaimer__details" data-alex-product-component="remote-connect-disclaimer" data-alex-product-part="details" summary={t('remoteConnect.disclaimerFullDetails')}>
            <ScrollArea className="alex-remote-disclaimer__list-scroll">
              <ol className="alex-remote-disclaimer__list" start={5}>
                <li>{t('remoteConnect.disclaimerItemOpenSource')}</li>
                <li>{t('remoteConnect.disclaimerItemDataUsage')}</li>
                <li>{t('remoteConnect.disclaimerItemCredentials')}</li>
                <li>{t('remoteConnect.disclaimerItemQrCode')}</li>
                <li>{t('remoteConnect.disclaimerItemRelay')}</li>
                <li>{t('remoteConnect.disclaimerItemNetwork')}</li>
                <li>{t('remoteConnect.disclaimerItemBot')}</li>
                <li>{t('remoteConnect.disclaimerItemBotPersistence')}</li>
                <li>{t('remoteConnect.disclaimerItemMobileBrowser')}</li>
                <li>{t('remoteConnect.disclaimerItemCompliance')}</li>
                <li>{t('remoteConnect.disclaimerItemLiability')}</li>
              </ol>
            </ScrollArea>
          </Disclosure>
        </div>
      </DialogBody>

      <DialogFooter
        separator
        className="alex-remote-disclaimer__actions"
        data-alex-product-component="remote-connect-disclaimer"
        data-alex-product-part="actions"
      >
        <Button
          className="alex-remote-disclaimer__action"
          variant={canAgree ? 'fill' : 'primary'}
          size="sm"
          onClick={onClose}
        >
          {canAgree ? t('remoteConnect.disclaimerDecline') : t('actions.close')}
        </Button>
        {canAgree && (
          <Button
            className="alex-remote-disclaimer__action"
            variant="primary"
            size="sm"
            onClick={onAgree}
            data-testid="remote-connect-disclaimer-agree"
          >
            {t('remoteConnect.disclaimerAgree')}
          </Button>
        )}
      </DialogFooter>
    </>
  );
};
