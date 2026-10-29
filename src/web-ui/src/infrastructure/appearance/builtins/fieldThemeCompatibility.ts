import type { AppearanceThemeTokenName } from '../types';

// Old packages used content.muted for hints and borderFocus for editing. Explicit field colors
// take precedence; missing values continue to come from the selected base theme.
export function withLegacyFieldTokens(
  tokens: Partial<Record<AppearanceThemeTokenName, string>> | undefined,
): Partial<Record<AppearanceThemeTokenName, string>> {
  const result = { ...tokens };
  const legacy = tokens?.['--alex-color-content-muted'];
  if (result['--alex-color-field-placeholder'] === undefined && legacy !== undefined) {
    result['--alex-color-field-placeholder'] = legacy;
  }
  const legacyFocus = tokens?.['--alex-color-field-border-focus'];
  if (result['--alex-color-field-border-active'] === undefined && legacyFocus !== undefined) {
    result['--alex-color-field-border-active'] = legacyFocus;
  }
  const legacyGroup = tokens?.['--alex-color-surface-tertiary'];
  if (result['--alex-color-field-group-background'] === undefined && legacyGroup !== undefined) {
    result['--alex-color-field-group-background'] = legacyGroup;
  }
  return result;
}
