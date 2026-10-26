import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const reasoningPresetSelectorAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'reasoning-preset-selector',
  parts: [
    { id: 'root' },
    { id: 'trigger' },
    { id: 'menu' },
    { id: 'header' },
    { id: 'auto' },
    { id: 'options' },
    { id: 'option' },
  ],
  facets: [
    {
      id: 'presentation',
      attribute: 'data-alex-presentation',
      values: ['meter', 'label'],
    },
  ],
  states: [
    { id: 'open', selector: { kind: 'self', suffix: '[data-alex-state~="open"]' } },
    { id: 'selected', selector: { kind: 'self', suffix: '[data-alex-state~="selected"]' } },
  ],
};
