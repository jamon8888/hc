import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const harnessProfileSelectorAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'harness-selector',
  parts: [
    { id: 'root' },
    { id: 'trigger' },
    { id: 'menu' },
    { id: 'profile' },
    { id: 'agent' },
  ],
  facets: [
    {
      id: 'presentation',
      attribute: 'data-alex-presentation',
      values: ['standalone', 'menu-item'],
    },
    {
      id: 'profile',
      attribute: 'data-alex-profile',
      values: ['Minimal', 'Standard', 'Ultimate', 'Creative', 'other'],
    },
  ],
  states: [
    { id: 'open', selector: { kind: 'self', suffix: '[data-alex-state~="open"]' } },
    { id: 'fixed', selector: { kind: 'self', suffix: '[data-alex-state~="fixed"]' } },
    { id: 'current', selector: { kind: 'self', suffix: '[data-alex-state~="current"]' } },
    { id: 'available', selector: { kind: 'self', suffix: '[data-alex-state~="available"]' } },
    { id: 'unavailable', selector: { kind: 'self', suffix: '[data-alex-state~="unavailable"]' } },
    { id: 'comingSoon', selector: { kind: 'self', suffix: '[data-alex-state~="coming-soon"]' } },
  ],
};
