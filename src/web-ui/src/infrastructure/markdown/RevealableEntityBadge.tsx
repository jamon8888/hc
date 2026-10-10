import React, { useState, useCallback } from 'react';
import { Tooltip } from '@openbitfun/ui';
import { basemindAPI } from '@/infrastructure/api';
import './RevealableEntityBadge.scss';

interface RevealableEntityBadgeProps {
  entityType: string;
  entityId: string;
  maskedLabel: React.ReactNode;
}

const ENTITY_TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',    // blue
  DATE: '#9ca3af',      // gray
  ORG: '#22c55e',       // green
  IBAN: '#ef4444',      // red
  EMAIL: '#a855f7',     // purple
  PHONE: '#f97316',     // orange
  ADDRESS: '#84cc16',   // lime
  LOCATION: '#06b6d4',  // cyan
  CITY: '#06b6d4',      // cyan
  POSTAL_CODE: '#84cc16', // lime
  NATIONAL_ID: '#ef4444', // red
  PASSPORT: '#ef4444',  // red
  CREDIT_CARD: '#ef4444', // red
  BANK_ACCOUNT: '#ef4444', // red
  DRIVERS_LICENSE: '#ef4444', // red
  TAX_ID: '#ef4444',    // red
  API_KEY: '#ef4444',   // red
  JWT: '#ef4444',       // red
  TOKEN: '#ef4444',     // red
  SSH_KEY: '#ef4444',   // red
  GPG_KEY: '#ef4444',   // red
  TLS_CERT: '#ef4444',  // red
  DB_CONNECTION: '#ef4444', // red
  ENV_SECRET: '#ef4444', // red
  IP_ADDRESS: '#6b7280', // gray
  INTERNAL_HOST: '#6b7280', // gray
  INTERNAL_URL: '#6b7280', // gray
  MAC_ADDRESS: '#6b7280', // gray
  COOKIE_ID: '#6b7280', // gray
  ORGANIZATION: '#22c55e', // green
  LOCATION: '#06b6d4', // cyan
};

const ENTITY_TYPE_LABELS: Record<string, string> = {
  PERSON: 'Person',
  DATE: 'Date',
  ORG: 'Organization',
  IBAN: 'IBAN',
  EMAIL: 'Email',
  PHONE: 'Phone',
  ADDRESS: 'Address',
  LOCATION: 'Location',
  CITY: 'City',
  POSTAL_CODE: 'Postal Code',
  NATIONAL_ID: 'National ID',
  PASSPORT: 'Passport',
  CREDIT_CARD: 'Credit Card',
  BANK_ACCOUNT: 'Bank Account',
  DRIVERS_LICENSE: "Driver's License",
  TAX_ID: 'Tax ID',
  API_KEY: 'API Key',
  JWT: 'JWT',
  TOKEN: 'Token',
  SSH_KEY: 'SSH Key',
  GPG_KEY: 'GPG Key',
  TLS_CERT: 'TLS Certificate',
  DB_CONNECTION: 'DB Connection',
  ENV_SECRET: 'Env Secret',
  IP_ADDRESS: 'IP Address',
  INTERNAL_HOST: 'Internal Host',
  INTERNAL_URL: 'Internal URL',
  MAC_ADDRESS: 'MAC Address',
  COOKIE_ID: 'Cookie ID',
  ORGANIZATION: 'Organization',
  LOCATION: 'Location',
};

export const RevealableEntityBadge: React.FC<RevealableEntityBadgeProps> = ({
  entityType,
  entityId,
  maskedLabel,
}) => {
  const [isRevealed, setIsRevealed] = useState(false);
  const [revealedValue, setRevealedValue] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const color = ENTITY_TYPE_COLORS[entityType] || '#6b7280';
  const label = ENTITY_TYPE_LABELS[entityType] || entityType;

  const handleClick = useCallback(async () => {
    if (isRevealed) {
      setIsRevealed(false);
      setRevealedValue(null);
      return;
    }

    setIsLoading(true);
    try {
      const value = await basemindAPI.revealEntity(entityId);
      setRevealedValue(value);
      setIsRevealed(true);
    } catch (error) {
      console.error('Failed to reveal entity:', error);
    } finally {
      setIsLoading(false);
    }
  }, [entityId, isRevealed]);

  const displayText = isRevealed && revealedValue ? revealedValue : maskedLabel;

  return (
    <Tooltip
      content={`${label}: ${isRevealed ? 'Click to hide' : 'Click to reveal'}`}
      placement="top"
      delay={200}
    >
      <span
        className="pii-entity-badge"
        style={{
          backgroundColor: color + '20',
          borderColor: color,
          color: color,
          border: '1px solid',
          borderRadius: '4px',
          padding: '2px 6px',
          fontSize: '0.85em',
          fontWeight: 500,
          cursor: 'pointer',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          transition: 'all 0.2s ease',
        } as React.CSSProperties}
        onClick={handleClick}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            handleClick();
          }}
        tabIndex={0}
        role="button"
        aria-label={`${label}: ${isRevealed ? 'Click to hide' : 'Click to reveal'}`}
      >
        <span style={{ textDecoration: isRevealed ? 'underline' : 'none' }}>
          {displayText}
        </span>
        {isLoading && (
          <span className="pii-badge-spinner" style={{ width: '10px', height: '10px', border: '2px solid currentColor', borderTopColor: 'transparent', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
        )}
        {!isLoading && (
          <span style={{ fontSize: '0.7em', opacity: 0.7 }}>
            {isRevealed ? '🔓' : '🔒'}
          </span>}
        </span>
    </Tooltip>
  );
};

export default RevealableEntityBadge;