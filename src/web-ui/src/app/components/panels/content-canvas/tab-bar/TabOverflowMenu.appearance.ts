import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';
export const canvasTabOverflowAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'canvas-tab-overflow',
  componentAttribute: 'data-alex-product-component',
  parts: [
    { id: 'root' }, { id: 'trigger' }, { id: 'badge' }, { id: 'itemTitle' },
  ],
  states: [
    { id: 'open', selector: { kind: 'self', suffix: '[data-alex-state~="open"]' } },
    { id: 'active', selector: { kind: 'self', suffix: '[data-alex-state~="active"]' } },
    { id: 'dirty', selector: { kind: 'self', suffix: '[data-alex-state~="dirty"]' } },
    { id: 'deleted', selector: { kind: 'self', suffix: '[data-alex-state~="deleted"]' } },
  ],
};
