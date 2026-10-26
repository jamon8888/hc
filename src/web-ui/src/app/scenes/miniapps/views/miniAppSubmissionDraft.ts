import type { MarketSubmissionDraftRequest } from '@/infrastructure/api/service-api/MiniAppMarketAPI';

export function createEmptyMarketSubmissionDraft(): MarketSubmissionDraftRequest {
  return {
    slug: '',
    releaseNumber: 1,
    name: '',
    description: '',
    icon: 'box',
    category: 'other',
    tags: [],
    minAlexVersion: '',
    changelog: '',
    license: { spdxExpression: 'MIT' },
  };
}

export function applyCurrentClientVersionDefault(
  draft: MarketSubmissionDraftRequest,
  currentClientVersion: string,
): MarketSubmissionDraftRequest {
  if (draft.minAlexVersion.trim() || !currentClientVersion.trim()) {
    return draft;
  }
  return {
    ...draft,
    minAlexVersion: currentClientVersion.trim(),
  };
}
