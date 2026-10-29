import type { AppearanceThemeTokenName } from '../types';

export function withLegacyIndicatorTokens(
  tokens: Partial<Record<AppearanceThemeTokenName, string>> | undefined,
): Partial<Record<AppearanceThemeTokenName, string>> {
  const result = { ...tokens };
  for (const [next, previous] of [
    ['--alex-color-number-badge-background', '--alex-color-action-neutral-surface'],
    ['--alex-color-key-hint-content', '--alex-color-content-muted'],
  ] as const) {
    if (result[next] === undefined && tokens?.[previous] !== undefined) result[next] = tokens[previous];
  }
  return result;
}
