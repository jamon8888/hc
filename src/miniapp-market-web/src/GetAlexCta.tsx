import { ArrowRight, Download as DownloadSimple } from 'lucide-react';
import type { MessageKey } from './i18n';
import { ALEX_DOWNLOAD_URL } from './links';

// A .bfminiapp package is useless without the desktop client, so the catalog
// and every listing keep one explicit path to the official download page.
export type GetAlexPlacement = 'catalog' | 'listing';

export function GetAlexCta({
  placement,
  t,
}: {
  placement: GetAlexPlacement;
  t: (key: MessageKey) => string;
}) {
  return (
    <a
      className={`get-alex get-alex-${placement}`}
      href={ALEX_DOWNLOAD_URL}
      target="_blank"
      rel="noreferrer"
    >
      <span className="get-alex-icon">
        <DownloadSimple size={18} aria-hidden="true" />
      </span>
      <span className="get-alex-copy">
        <strong>{t('getAlexTitle')}</strong>
        <span>
          {t(placement === 'listing' ? 'getAlexListingNote' : 'getAlexCatalogNote')}
        </span>
        <span className="get-alex-action">
          {t('getAlexAction')}
          <ArrowRight size={18} aria-hidden="true" />
        </span>
      </span>
    </a>
  );
}
