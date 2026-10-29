import type { AppearanceThemeTokenName } from '../types';

export function withLegacyCaptionToken(
  tokens: Partial<Record<AppearanceThemeTokenName, string>> | undefined,
): Partial<Record<AppearanceThemeTokenName, string>> {
  const result = { ...tokens };
  if (result['--alex-color-content-caption'] === undefined
    && tokens?.['--alex-color-content-muted'] !== undefined) {
    result['--alex-color-content-caption'] = tokens['--alex-color-content-muted'];
  }
  return result;
}
