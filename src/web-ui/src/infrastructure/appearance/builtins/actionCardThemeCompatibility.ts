import type { AppearanceThemeTokenName } from '../types';

export function withLegacyActionCardToken(
  tokens: Partial<Record<AppearanceThemeTokenName, string>> | undefined,
): Partial<Record<AppearanceThemeTokenName, string>> {
  const result = { ...tokens };
  if (result['--alex-color-action-card-background'] === undefined
    && tokens?.['--alex-color-action-neutral-surface'] !== undefined) {
    result['--alex-color-action-card-background'] = tokens['--alex-color-action-neutral-surface'];
  }
  return result;
}
