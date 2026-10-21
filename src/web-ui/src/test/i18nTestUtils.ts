import { vi } from 'vitest';

import { i18nService } from '@/infrastructure/i18n/core/I18nService';
import type { LocaleId } from '@/infrastructure/i18n/presets';
import commonMessages from '@/locales/en-US/common.json';
import componentsMessages from '@/locales/en-US/components.json';
import flowChatMessages from '@/locales/en-US/flow-chat.json';

type TranslationOptions = Record<string, unknown> & {
  defaultValue?: string;
};

type TranslationKey = string | readonly string[];

const resources: Record<string, unknown> = {
  common: commonMessages,
  components: componentsMessages,
  'flow-chat': flowChatMessages,
};

export function createTestI18nT(defaultNamespace = 'common') {
  return (key: TranslationKey, options?: TranslationOptions): string => {
    const keys = Array.isArray(key) ? key : [key];
    for (const candidate of keys) {
      const value = resolveMessage(candidate, defaultNamespace);
      if (typeof value === 'string') {
        return interpolate(value, options);
      }
    }

    if (typeof options?.defaultValue === 'string') {
      return interpolate(options.defaultValue, options);
    }

    return String(keys[0] ?? '');
  };
}

function resolveMessage(key: string, defaultNamespace: string): unknown {
  const separatorIndex = key.indexOf(':');
  const namespace = separatorIndex > 0 ? key.slice(0, separatorIndex) : defaultNamespace;
  const path = separatorIndex > 0 ? key.slice(separatorIndex + 1) : key;
  return getPath(resources[namespace], path);
}

function getPath(source: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((current, segment) => {
    if (current && typeof current === 'object' && segment in current) {
      return (current as Record<string, unknown>)[segment];
    }
    return undefined;
  }, source);
}

function interpolate(message: string, options?: TranslationOptions): string {
  return message.replace(/\{\{(\w+)\}\}/g, (match, token: string) => {
    const value = options?.[token];
    return value == null ? match : String(value);
  });
}

/**
 * Pins the shared number formatting to one locale.
 *
 * The product default locale is a product decision, so a test that asserts
 * formatted numbers has to state the locale it asserts against instead of
 * inheriting the ambient default. The formatting goes through the i18next
 * formatter service so the locale stays explicit without a direct `Intl` call.
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

/**
 * Pins the i18n catalog to one locale.
 *
 * Use this when the assertion proves that the copy came from the catalog
 * rather than from the raw source text: only a locale other than the source
 * language can tell the two apart.
 */
export async function pinCatalogLocale(locale: LocaleId): Promise<void> {
  await i18nService.getI18nInstance().changeLanguage(locale);
}
