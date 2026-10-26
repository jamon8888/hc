/**
 * Token usage indicator.
 * Shows session token usage as percentage and count.
 */

import React, { useMemo } from 'react';
import { useI18n } from '@/infrastructure/i18n';
import './TokenUsageIndicator.scss';

export interface TokenUsageIndicatorProps {
  currentTokens: number;    // Current token usage
  maxTokens: number;        // Max token capacity
  className?: string;
}

export const TokenUsageIndicator: React.FC<TokenUsageIndicatorProps> = ({
  currentTokens,
  maxTokens,
  className = ''
}) => {
  const { formatNumber } = useI18n();
  const percentage = useMemo(() => {
    if (!maxTokens || maxTokens <= 0) return 0;
    return Math.min(Math.round((currentTokens / maxTokens) * 100), 100);
  }, [currentTokens, maxTokens]);

  const getStatusClass = (percent: number): string => {
    if (percent >= 90) return 'critical';
    if (percent >= 70) return 'warning';
    return 'normal';
  };

  const statusClass = getStatusClass(percentage);

  return (
    <div
      data-alex-component="token-usage-indicator"
      data-alex-part="root"
      data-alex-level={statusClass}
      className={`alex-token-usage ${className} alex-token-usage--${statusClass}`}
      title={`Token usage: ${formatNumber(currentTokens)} / ${formatNumber(maxTokens)}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percentage}
    >
      <div data-alex-component="token-usage-indicator" data-alex-part="track" className="alex-token-usage__progress-track">
        <div 
          data-alex-component="token-usage-indicator"
          data-alex-part="fill"
          className="alex-token-usage__progress-fill"
          style={{ '--token-usage-progress': percentage / 100 } as React.CSSProperties}
        />
      </div>
      
      <div data-alex-component="token-usage-indicator" data-alex-part="hover" className="alex-token-usage__hover-content">
        <span data-alex-component="token-usage-indicator" data-alex-part="percentage" className="alex-token-usage__percentage">{percentage}%</span>
      </div>
    </div>
  );
};

export default TokenUsageIndicator;
