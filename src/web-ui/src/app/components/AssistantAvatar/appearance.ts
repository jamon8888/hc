import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const assistantAvatarAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'assistant-avatar',
  parts: [{ id: 'root' }],
  states: [
    { id: 'active', selector: { kind: 'self', suffix: '[data-alex-state~="active"]' } },
    { id: 'running', selector: { kind: 'self', suffix: '[data-alex-state~="running"]' } },
    { id: 'attention', selector: { kind: 'self', suffix: '[data-alex-state~="attention"]' } },
    { id: 'unread', selector: { kind: 'self', suffix: '[data-alex-state~="unread"]' } },
    { id: 'error', selector: { kind: 'self', suffix: '[data-alex-state~="error"]' } },
  ],
};
