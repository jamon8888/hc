import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

/**
 * Revealable PII entity badge.
 *
 * The badge lives inside rendered Markdown, so it owns a small surface of its
 * own: the badge itself, the masked label, the reveal icon, and the spinner
 * that replaces it while the reveal is in flight.
 */
export const revealableEntityBadgeAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'pii-entity-badge',
  parts: [
    { id: 'root' },
    { id: 'label' },
    { id: 'icon' },
    { id: 'spinner' },
  ],
  states: [
    { id: 'loading', selector: { kind: 'self', suffix: '[data-openbitfun-state~="loading"]' } },
    { id: 'revealed', selector: { kind: 'self', suffix: '[data-openbitfun-state~="revealed"]' } },
    { id: 'error', selector: { kind: 'self', suffix: '[data-openbitfun-state~="error"]' } },
  ],
};
