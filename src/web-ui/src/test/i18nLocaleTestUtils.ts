import { vi } from 'vitest';

import { i18nService } from '@/infrastructure/i18n/core/I18nService';
import type { LocaleId } from '@/infrastructure/i18n/presets';

/**
 * Pins the shared number formatting to one locale.
 *
 * The product default locale is a product decision, so a test that asserts
 * formatted numbers has to state the locale it asserts against instead of
 * inheriting the ambient default. The formatting goes through the i18next
 * formatter service so the locale stays explicit without a direct `Intl` call.
 *
 * This helper lives apart from `i18nTestUtils` on purpose: that module is
 * shared by many test files, and pulling the i18n service into all of them
 * changes what every one of them loads.
 */
export function pinNumberFormattingLocale(locale: LocaleId = 'en-US'): void {
  const formatter = i18nService.getI18nInstance().services.formatter;
  if (!formatter) {
    throw new Error('The i18next formatter service is unavailable');
  }
  vi.spyOn(i18nService, 'formatNumber').mockImplementation(
    (value, options) => formatter.format(value, 'number', locale, options),
  );
}
