import React, { useState, useCallback } from 'react';
import { Tooltip } from '@openbitfun/ui';
import { Eye, EyeOff } from 'lucide-react';
import { basemindAPI } from '@/infrastructure/api';
import { useI18n } from '@/infrastructure/i18n';
import { createLogger } from '@/shared/utils/logger';
import './RevealableEntityBadge.scss';

const log = createLogger('RevealableEntityBadge');

/**
 * Semantic tone of an entity type.
 *
 * The redaction surface reports a type per entity, and the tone decides which
 * theme role colors it. Identity data reads as accent, contact data as
 * accent-secondary, secrets and financial identifiers as danger, and neutral
 * metadata as muted content. Colors stay theme-owned instead of hard-coded
 * per type.
 */
export type EntityTone = 'identity' | 'contact' | 'secret' | 'neutral';

const ENTITY_TONES: Record<string, EntityTone> = {
  PERSON: 'identity',
  ORG: 'identity',
  ORGANIZATION: 'identity',
  ADDRESS: 'identity',
  LOCATION: 'identity',
  CITY: 'identity',
  EMAIL: 'contact',
  PHONE: 'contact',
  IBAN: 'secret',
  NATIONAL_ID: 'secret',
  PASSPORT: 'secret',
  CREDIT_CARD: 'secret',
  BANK_ACCOUNT: 'secret',
  DRIVERS_LICENSE: 'secret',
  TAX_ID: 'secret',
  API_KEY: 'secret',
  JWT: 'secret',
  TOKEN: 'secret',
  SSH_KEY: 'secret',
  GPG_KEY: 'secret',
  TLS_CERT: 'secret',
  DB_CONNECTION: 'secret',
  ENV_SECRET: 'secret',
  DATE: 'neutral',
  POSTAL_CODE: 'neutral',
  IP_ADDRESS: 'neutral',
  INTERNAL_HOST: 'neutral',
  INTERNAL_URL: 'neutral',
  MAC_ADDRESS: 'neutral',
  COOKIE_ID: 'neutral',
};

export interface RevealableEntityBadgeProps {
  entityType: string;
  entityId: string;
  maskedLabel: React.ReactNode;
  /** Owning workspace; the reveal command runs against that workspace. */
  workspaceId?: string;
}

export const RevealableEntityBadge: React.FC<RevealableEntityBadgeProps> = ({
  entityType,
  entityId,
  maskedLabel,
  workspaceId,
}) => {
  const { t } = useI18n('panels/redacted-document');
  const [isRevealed, setIsRevealed] = useState(false);
  const [revealedValue, setRevealedValue] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const tone = ENTITY_TONES[entityType] ?? 'neutral';
  const label = t(`entityTypes.${entityType}`, { defaultValue: entityType });
  const actionLabel = isRevealed ? t('badge.hide') : t('badge.reveal');

  const handleClick = useCallback(async () => {
    if (isLoading) {
      return;
    }
    if (isRevealed) {
      setIsRevealed(false);
      setRevealedValue(null);
      setErrorMessage(null);
      return;
    }

    setIsLoading(true);
    setErrorMessage(null);
    try {
      const value = await basemindAPI.revealEntity(entityId, workspaceId);
      setRevealedValue(value);
      setIsRevealed(true);
    } catch (error) {
      const cause = error instanceof Error ? error : new Error(String(error));
      log.error('Failed to reveal redacted entity', { entityId, entityType, error: cause });
      setErrorMessage(cause.message || t('state.revealFailed'));
    } finally {
      setIsLoading(false);
    }
  }, [entityId, entityType, isLoading, isRevealed, t, workspaceId]);

  const displayText = isRevealed && revealedValue ? revealedValue : maskedLabel;
  const statusText = errorMessage ?? `${label}: ${actionLabel}`;

  return (
    <Tooltip
      content={statusText}
      placement="top"
      delay={200}
    >
      <span
        className={`pii-entity-badge pii-entity-badge--${tone}`}
        data-openbitfun-component="pii-entity-badge"
        data-openbitfun-part="root"
        data-openbitfun-state={errorMessage ? 'error' : isRevealed ? 'revealed' : isLoading ? 'loading' : undefined}
        data-entity-type={entityType}
        onClick={() => { void handleClick(); }}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            void handleClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label={statusText}
      >
        <span className="pii-entity-badge__label" data-openbitfun-component="pii-entity-badge" data-openbitfun-part="label">
          {displayText}
        </span>
        {isLoading && (
          <span
            className="pii-entity-badge__spinner"
            data-openbitfun-component="pii-entity-badge"
            data-openbitfun-part="spinner"
            aria-hidden="true"
          />
        )}
        {!isLoading && (isRevealed
          ? <EyeOff className="pii-entity-badge__icon" data-openbitfun-component="pii-entity-badge" data-openbitfun-part="icon" size={12} aria-hidden="true" />
          : <Eye className="pii-entity-badge__icon" data-openbitfun-component="pii-entity-badge" data-openbitfun-part="icon" size={12} aria-hidden="true" />)}
      </span>
    </Tooltip>
  );
};

export default RevealableEntityBadge;
