import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

/**
 * Redacted document viewer surface.
 *
 * The viewer owns the tab bar, the source pane, and the redacted pane, so
 * each is a distinct part and an Appearance can restyle them separately.
 */
export const redactedDocumentTabsAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'redacted-document-tabs',
  parts: [
    { id: 'root' },
    { id: 'toolbar' },
    { id: 'tab' },
    { id: 'summary' },
    { id: 'content' },
    { id: 'source' },
    { id: 'redacted' },
    { id: 'loading' },
    { id: 'notice' },
  ],
  states: [
    { id: 'loading', selector: { kind: 'self', suffix: '[data-openbitfun-state~="loading"]' } },
    { id: 'error', selector: { kind: 'self', suffix: '[data-openbitfun-state~="error"]' } },
  ],
};
