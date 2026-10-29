import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance';

export const realtimeVoiceCallAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'realtime-voice-call',
  componentAttribute: 'data-alex-product-component',
  parts: [
    { id: 'root' },
  ],
  facets: [
    {
      id: 'phase',
      attribute: 'data-alex-phase',
      values: ['connecting', 'live', 'ending', 'error'],
    },
  ],
  states: [
    { id: 'idle', selector: { kind: 'self', suffix: '[data-alex-state~="idle"]' } },
    { id: 'connecting', selector: { kind: 'self', suffix: '[data-alex-state~="connecting"]' } },
    { id: 'live', selector: { kind: 'self', suffix: '[data-alex-state~="live"]' } },
    { id: 'ending', selector: { kind: 'self', suffix: '[data-alex-state~="ending"]' } },
    { id: 'error', selector: { kind: 'self', suffix: '[data-alex-state~="error"]' } },
    { id: 'starting', selector: { kind: 'self', suffix: '[data-alex-state~="starting"]' } },
    { id: 'working', selector: { kind: 'self', suffix: '[data-alex-state~="working"]' } },
    { id: 'usingTools', selector: { kind: 'self', suffix: '[data-alex-state~="using-tools"]' } },
    { id: 'waitingApproval', selector: { kind: 'self', suffix: '[data-alex-state~="waiting-approval"]' } },
    { id: 'needsInput', selector: { kind: 'self', suffix: '[data-alex-state~="needs-input"]' } },
    { id: 'finishing', selector: { kind: 'self', suffix: '[data-alex-state~="finishing"]' } },
    { id: 'stopping', selector: { kind: 'self', suffix: '[data-alex-state~="stopping"]' } },
  ],
};
