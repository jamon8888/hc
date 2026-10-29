import { Button } from '@alex/ui';
import React from 'react';

import { ConfigMessage } from './ConfigMessage';
import './ConfigPageState.scss';

export interface ConfigRetryStateProps {
  message: string;
  retryLabel: string;
  onRetry: () => void;
  loading?: boolean;
  className?: string;
}

/**
 * Persistent failed-read state for settings surfaces.
 *
 * Callers should render this instead of editable fallback values when no
 * trusted configuration snapshot is available.
 */
export const ConfigRetryState: React.FC<ConfigRetryStateProps> = ({
  message,
  retryLabel,
  onRetry,
  loading = false,
  className = '',
}) => (
  <div
    className={['alex-config-retry-state', className].filter(Boolean).join(' ')}
    data-alex-component="config"
    data-alex-part="retryState"
  >
    <ConfigMessage message={{ type: 'error', text: message }} />
    <Button
      type="button"
      variant="outline"
      size="sm"
      onClick={onRetry}
      loading={loading}
      disabled={loading}
    >
      {retryLabel}
    </Button>
  </div>
);
