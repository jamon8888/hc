import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const scrollToTurnHeaderButtonAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'scroll-to-turn-header-button',
  componentAttribute: 'data-alex-product-component',
  parts: [{ id: 'root' }, { id: 'gradient' }, { id: 'content' }, { id: 'button' }],
  states: [{ id: 'visible', selector: { kind: 'self', suffix: '[data-alex-state~="visible"]' } }],
};
