import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { GetAlexCta, type GetAlexPlacement } from './GetAlexCta';
import type { Translate } from './i18n';
import { ALEX_DOWNLOAD_URL } from './links';

const t = ((key: string) => key) as Translate;

describe('GetAlexCta', () => {
  it.each<GetAlexPlacement>(['catalog', 'listing'])(
    'sends %s visitors to the official download page',
    (placement) => {
      const markup = renderToStaticMarkup(<GetAlexCta placement={placement} t={t} />);

      expect(markup).toContain(`href="${ALEX_DOWNLOAD_URL}"`);
      expect(markup).toContain('rel="noreferrer"');
      expect(markup).toContain('getAlexTitle');
      expect(markup).toContain('getAlexAction');
    },
  );

  it('explains the surface the visitor is actually looking at', () => {
    expect(renderToStaticMarkup(<GetAlexCta placement="listing" t={t} />))
      .toContain('getAlexListingNote');
    expect(renderToStaticMarkup(<GetAlexCta placement="catalog" t={t} />))
      .toContain('getAlexCatalogNote');
  });
});
