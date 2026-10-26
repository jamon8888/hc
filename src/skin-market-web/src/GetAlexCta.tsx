import { ArrowRight, Download as DownloadSimple } from 'lucide-react';
import type { Translate } from './i18n';
import { ALEX_DOWNLOAD_URL } from './links';

// An appearance package only applies inside the desktop client, so the catalog
// and every listing carry the same visible route to the official download page.
export type GetAlexPlacement = 'catalog' | 'listing';

export function GetAlexCta({
  placement,
  t,
}: {
  placement: GetAlexPlacement;
  t: Translate;
}) {
  return (
    <a
      className={`get-alex get-alex--${placement}`}
      href={ALEX_DOWNLOAD_URL}
      target="_blank"
      rel="noreferrer"
    >
      <span className="get-alex__icon">
        <DownloadSimple size={20} aria-hidden="true" />
      </span>
      <span className="get-alex__copy">
        <strong>{t('getAlexTitle')}</strong>
        <span>
          {t(placement === 'listing' ? 'getAlexListingNote' : 'getAlexCatalogNote')}
        </span>
        <span className="get-alex__action">
          {t('getAlexAction')}
          <ArrowRight size={17} aria-hidden="true" />
        </span>
      </span>
    </a>
  );
}
