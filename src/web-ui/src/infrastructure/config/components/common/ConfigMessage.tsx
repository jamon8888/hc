import React from 'react';
import { Alert } from '@alex/ui';
import './ConfigPageState.scss';

export interface ConfigMessageData {
  type: 'success' | 'error' | 'info' | 'warning';
  text: string;
}

export interface ConfigMessageProps {
  message: ConfigMessageData | null;
  className?: string;
}

export const ConfigMessage: React.FC<ConfigMessageProps> = ({
  message,
  className = '',
}) => {
  if (!message) return null;

  return (
    <div
      className={['alex-config-message', className].filter(Boolean).join(' ')}
      data-alex-component="config"
      data-alex-part="message"
    >
      <Alert tone={message.type} message={message.text} />
    </div>
  );
};
