import type { AppearanceThemeTokenName } from '../types';

export function withLegacyComposerTokens(
  tokens: Partial<Record<AppearanceThemeTokenName, string>> | undefined,
): Partial<Record<AppearanceThemeTokenName, string>> {
  const result = { ...tokens };
  for (const [next, previous] of [
    ['--alex-color-composer-border', '--alex-color-field-border'],
    ['--alex-color-composer-context-background', '--alex-color-surface-subtle'],
  ] as const) {
    if (result[next] === undefined && tokens?.[previous] !== undefined) result[next] = tokens[previous];
  }
  return result;
}
