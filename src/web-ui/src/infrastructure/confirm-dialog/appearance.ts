import type { AppearanceSurfaceDescriptor } from '@/infrastructure/appearance/types';

export const confirmDialogAppearanceDescriptor: AppearanceSurfaceDescriptor = {
  id: 'confirm-dialog',
  parts: [
    { id: 'content' },
    { id: 'messageRow' },
    { id: 'icon' },
    { id: 'message' },
    { id: 'preview' },
  ],
  facets: [{
    id: 'status',
    attribute: 'data-alex-status',
    values: ['info', 'warning', 'danger', 'success'],
  }],
};
