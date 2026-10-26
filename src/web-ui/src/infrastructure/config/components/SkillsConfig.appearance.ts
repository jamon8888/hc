import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const skillsConfigAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'skills-config',
  parts: [
    { id: 'root' }, { id: 'content' }, { id: 'form' }, { id: 'formBody' },
    { id: 'item' }, { id: 'details' }, { id: 'marketToolbar' },
    { id: 'marketList' }, { id: 'marketItem' }, { id: 'marketState' },
    { id: 'empty' }, { id: 'loading' }, { id: 'error' },
  ],
  states: [
    { id: 'loading', selector: { kind: 'self', suffix: '[data-alex-state~="loading"]' } },
    { id: 'error', selector: { kind: 'self', suffix: '[data-alex-state~="error"]' } },
    { id: 'installed', selector: { kind: 'self', suffix: '[data-alex-state~="installed"]' } },
    { id: 'covered', selector: { kind: 'self', suffix: '[data-alex-state~="covered"]' } },
  ],
};
