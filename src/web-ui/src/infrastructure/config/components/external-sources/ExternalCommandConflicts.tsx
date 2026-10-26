import { Button } from '@alex/ui';
import React from 'react';
import type { ExternalSourceCatalogSnapshot } from '@/infrastructure/api/service-api/ExternalSourcesAPI';
import { ConfigPageSection } from '../common';
import { abbreviatedLocation, sourceScopeLabel } from './presentation';
import type { ExternalSectionCommonProps } from './types';

type CommandConflict = NonNullable<ExternalSourceCatalogSnapshot['commandConflicts']>[number];

export interface ExternalCommandConflictsProps
  extends Omit<ExternalSectionCommonProps, 'snapshot' | 'renderPathLink'> {
  conflicts: CommandConflict[];
  onChooseConflict: (conflictKey: string, candidateId: string) => void;
}

/**
 * Prompt command name collisions across ecosystems. Selection stays a user
 * decision: the controller never resolves these by registration order.
 */
export const ExternalCommandConflicts: React.FC<ExternalCommandConflictsProps> = ({
  conflicts,
  t,
  busyKey,
  hostCapabilities,
  policyCompatible,
  onChooseConflict,
}) => {
  if (conflicts.length === 0) return null;

  return (
    <ConfigPageSection
      title={t('conflicts.title')}
    >
      {conflicts.map((conflict) => {
        const ecosystemIds = new Set(
          conflict.candidates.map((candidate) => candidate.ecosystemId),
        );
        const selectedChoiceUnavailable = conflict.candidates.some((candidate) => (
          candidate.candidateId === conflict.selectedCandidateId
          && candidate.availability.state !== 'available'
        ));
        return (
          <div
            className="alex-external-sources-config__conflict"
            data-alex-product-component="external-sources-config"
            data-alex-product-part="conflict"
            key={conflict.conflictKey}
            data-external-attention={!conflict.selectedCandidateId ? 'true' : undefined}
            data-external-ecosystem={ecosystemIds.size === 1
              ? ecosystemIds.values().next().value
              : undefined}
          >
          <div className="alex-external-sources-config__conflict-title" data-alex-product-component="external-sources-config" data-alex-product-part="conflictTitle">
            {t('conflicts.commandName', { name: conflict.commandName })}
          </div>
          <div className="alex-external-sources-config__conflict-options" data-alex-product-component="external-sources-config" data-alex-product-part="conflictOptions">
            {conflict.candidates.map((candidate) => {
              const selected = conflict.selectedCandidateId === candidate.candidateId;
              const available = candidate.availability.state === 'available';
              return (
                <div
                  className="alex-external-sources-config__candidate"
                  data-alex-product-component="external-sources-config"
                  data-alex-product-part="candidate"
                  key={candidate.candidateId}
                >
                  <Button
                    variant={selected ? 'fill' : 'outline'}
                    size="sm"
                    disabled={!policyCompatible || busyKey === conflict.conflictKey || !available
                      || !hostCapabilities.canManageSources}
                    aria-pressed={selected}
                    onClick={() => void onChooseConflict(
                      conflict.conflictKey,
                      candidate.candidateId,
                    )}
                  >
                    {candidate.sourceDisplayName}
                    <span className="alex-external-sources-config__ecosystem">
                      {candidate.ecosystemId}
                    </span>
                  </Button>
                  <span className="alex-external-sources-config__candidate-state" data-alex-product-component="external-sources-config" data-alex-product-part="candidateState">
                    {t(selected
                      ? selectedChoiceUnavailable
                        ? 'common.selectedUnavailable'
                        : 'common.selected'
                      : !available
                        ? 'conflicts.restricted'
                        : conflict.selectedCandidateId
                          ? 'common.notSelected'
                          : 'common.availableChoice')}
                  </span>
                  <div className="alex-external-sources-config__candidate-detail" data-alex-product-component="external-sources-config" data-alex-product-part="candidateDetail">
                    {candidate.commandDescription}
                    {' · '}
                    {sourceScopeLabel(candidate.sourceScope, t)}
                    {' · '}
                    <span title={candidate.sourceLocation}>
                      {abbreviatedLocation(candidate.sourceLocation)}
                    </span>
                    {!available ? ` · ${t('conflicts.restricted')}` : ''}
                  </div>
                </div>
              );
            })}
          </div>
          <div className="alex-external-sources-config__conflict-hint" data-alex-product-component="external-sources-config" data-alex-product-part="conflictHint">
            {conflict.selectedCandidateId
              ? t(selectedChoiceUnavailable
                ? 'conflicts.currentSelectionUnavailable'
                : 'conflicts.currentSelection')
              : t('conflicts.pending')}
          </div>
          </div>
        );
      })}
    </ConfigPageSection>
  );
};
