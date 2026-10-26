import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const userMessageEditComposerAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'user-message-edit-composer',
  componentAttribute: 'data-alex-product-component',
  parts: [{ id: 'root' }, { id: 'input' }, { id: 'actions' }, { id: 'action' }, { id: 'spinner' }],
  facets: [
    { id: 'mode', attribute: 'data-alex-mode', values: ['rich', 'plain'] },
    { id: 'action', attribute: 'data-alex-action', values: ['cancel', 'submit'] },
  ],
  states: [
    { id: 'submitting', selector: { kind: 'ancestorPart', part: 'root', suffix: '[data-alex-state~="submitting"]' } },
  ],
};
