import { createHash } from 'node:crypto';
import {
  cssVariables as systemCssVariables,
  tokens as systemTokens,
} from '@alex/design-tokens';
import { themeCssVariables } from '@alex/theme-alex';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { widgetAppearanceAdapter } from '@/infrastructure/appearance/adapters/WidgetAppearanceAdapter';
import {
  WIDGET_APPEARANCE_FALLBACK_VARS,
  WIDGET_APPEARANCE_VAR_NAMES,
  WIDGET_TYPOGRAPHY_VARIABLE_NAMES,
  createWidgetAppearanceFallbackCss,
  readWidgetAppearancePayload,
} from './appearancePayload';

const CANONICAL_THEME_VARIABLE_NAMES = Object.values(themeCssVariables);
const SHARED_THEME_VARIABLE_NAMES_HASH = 'c1a064d75473584bd2c16d6c5c07b7903e242234ef9cec50e68a6e18561f8956';
// Button owns these state colors independently of the existing shared actions.
// Keep the original shared contract intact and enumerate this addition exactly.
const BUTTON_THEME_VARIABLE_NAMES = [
  '--alex-component-button-content',
  '--alex-component-button-fill-background',
  '--alex-component-button-fill-background-hover',
  '--alex-component-button-fill-background-pressed',
  '--alex-component-button-outline-border',
  '--alex-component-button-outline-border-interactive',
  '--alex-component-button-primary-background',
  '--alex-component-button-primary-background-hover',
  '--alex-component-button-primary-background-pressed',
  '--alex-component-button-primary-content-disabled',
  '--alex-component-button-text-content',
  '--alex-component-button-text-content-disabled',
  '--alex-component-button-text-content-hover',
] as const;
// Field editing borders and hints have dedicated semantics; preserve the shared
// contract fingerprint while asserting these two additions by their exact names.
const FIELD_STATE_THEME_VARIABLE_NAMES = [
  '--alex-color-field-border-active',
  '--alex-color-field-placeholder',
] as const;
// Grouped forms own a translucent fill independently of opaque tertiary surfaces
// and transient subtle feedback; keep this addition outside the shared fingerprint.
const FIELD_GROUP_THEME_VARIABLE_NAME = '--alex-color-field-group-background';
// Menu and navigation captions own their final contrast independently of body
// descriptions and field hints; assert this addition without changing the shared baseline.
const CAPTION_THEME_VARIABLE_NAME = '--alex-color-content-caption';
// Persistent action cards own their fill independently of transient feedback
// and form groups; keep the shared contract fingerprint unchanged.
const ACTION_CARD_THEME_VARIABLE_NAME = '--alex-color-action-card-background';
// Compact indicators own badge fill and key-hint contrast independently of
// shared action surfaces and muted prose; enumerate only these additions.
const INDICATOR_THEME_VARIABLE_NAMES = [
  '--alex-color-key-hint-content',
  '--alex-color-number-badge-background',
] as const;
// Composer and ChatComposer share editor borders and context tint independently
// of form fields and transient surfaces; enumerate these additions exactly.
const COMPOSER_THEME_VARIABLE_NAMES = [
  '--alex-color-composer-border',
  '--alex-color-composer-context-background',
] as const;
const RETIRED_WIDGET_VARIABLE_NAMES = [
  '--background-primary',
  '--bg-primary',
  '--text-primary',
  '--accent-primary',
  '--alex-appearance-token-color-bg-primary',
  '--alex-appearance-token-color-text-primary',
  '--alex-appearance-token-color-accent-500',
  '--alex-appearance-token-btn-primary-bg',
  '--alex-color-accent-default-rgb',
  '--alex-color-status-success-content-bg',
  '--alex-color-status-success-content-border',
  '--alex-color-status-warning-content-bg',
  '--alex-color-status-warning-content-border',
  '--alex-color-status-danger-content-bg',
  '--alex-color-status-danger-content-border',
] as const;

function readPayloadWithHostValues(hostValues: Record<string, string> = {}) {
  widgetAppearanceAdapter.apply({
    id: 'test-appearance',
    mode: 'dark',
    vars: hostValues,
  }, undefined, { revision: 1, mode: 'dark', assets: {} });

  return readWidgetAppearancePayload();
}

function hashNames(names: readonly string[]): string {
  return createHash('sha256')
    .update(names.join('\n'))
    .digest('hex');
}

describe('generated widget appearance payload contract', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('derives its complete host payload allowlist from the canonical theme package', () => {
    expect(WIDGET_APPEARANCE_VAR_NAMES).toEqual(CANONICAL_THEME_VARIABLE_NAMES);
    expect(new Set(WIDGET_APPEARANCE_VAR_NAMES).size).toBe(WIDGET_APPEARANCE_VAR_NAMES.length);
    const buttonNames = WIDGET_APPEARANCE_VAR_NAMES.filter(name => name.startsWith('--alex-component-button-'));
    const fieldStateNames = WIDGET_APPEARANCE_VAR_NAMES.filter(name => FIELD_STATE_THEME_VARIABLE_NAMES.some(fieldName => fieldName === name));
    const indicatorNames = WIDGET_APPEARANCE_VAR_NAMES.filter(name => INDICATOR_THEME_VARIABLE_NAMES.some(indicatorName => indicatorName === name));
    const composerNames = WIDGET_APPEARANCE_VAR_NAMES.filter(name => COMPOSER_THEME_VARIABLE_NAMES.some(composerName => composerName === name));
    const sharedNames = WIDGET_APPEARANCE_VAR_NAMES.filter(name => (
      !name.startsWith('--alex-component-button-')
      && !FIELD_STATE_THEME_VARIABLE_NAMES.some(fieldName => fieldName === name)
      && name !== FIELD_GROUP_THEME_VARIABLE_NAME
      && name !== CAPTION_THEME_VARIABLE_NAME
      && name !== ACTION_CARD_THEME_VARIABLE_NAME
      && !INDICATOR_THEME_VARIABLE_NAMES.some(indicatorName => indicatorName === name)
      && !COMPOSER_THEME_VARIABLE_NAMES.some(composerName => composerName === name)
    ));
    expect(buttonNames).toEqual(BUTTON_THEME_VARIABLE_NAMES);
    expect(fieldStateNames).toEqual(FIELD_STATE_THEME_VARIABLE_NAMES);
    expect(indicatorNames).toEqual(INDICATOR_THEME_VARIABLE_NAMES);
    expect(composerNames).toEqual(COMPOSER_THEME_VARIABLE_NAMES);
    expect(WIDGET_APPEARANCE_VAR_NAMES).toContain(FIELD_GROUP_THEME_VARIABLE_NAME);
    expect(WIDGET_APPEARANCE_VAR_NAMES).toContain(CAPTION_THEME_VARIABLE_NAME);
    expect(WIDGET_APPEARANCE_VAR_NAMES).toContain(ACTION_CARD_THEME_VARIABLE_NAME);
    expect({
      count: sharedNames.length,
      hash: hashNames(sharedNames),
      first: sharedNames[0],
      last: sharedNames[sharedNames.length - 1],
    }).toEqual({
      count: 128,
      hash: SHARED_THEME_VARIABLE_NAMES_HASH,
      first: '--alex-color-accent-border',
      last: '--alex-shadow-xs',
    });
  });

  it('uses the builtin canonical theme as the payload fallback', () => {
    const payload = readPayloadWithHostValues();

    expect(payload?.vars).toMatchObject(WIDGET_APPEARANCE_FALLBACK_VARS);
    expect(Object.keys(WIDGET_APPEARANCE_FALLBACK_VARS)).toEqual(WIDGET_APPEARANCE_VAR_NAMES);
    expect(Object.keys(payload?.vars ?? {}).sort()).toEqual([
      ...WIDGET_APPEARANCE_VAR_NAMES,
      ...WIDGET_TYPOGRAPHY_VARIABLE_NAMES,
    ].sort());
    expect(WIDGET_APPEARANCE_FALLBACK_VARS['--alex-color-surface-canvas']).toBe('transparent');
  });

  it('exports canonical state and status semantics without legacy aliases', () => {
    expect(WIDGET_APPEARANCE_VAR_NAMES).toEqual(expect.arrayContaining([
      '--alex-color-action-primary-background',
      '--alex-color-action-primary-hover',
      '--alex-color-action-primary-pressed',
      '--alex-color-code-change-added',
      '--alex-color-code-change-removed',
      '--alex-color-status-info-emphasis',
      '--alex-color-status-success-emphasis',
      '--alex-color-status-warning-emphasis',
      '--alex-color-status-danger-emphasis',
      '--alex-color-status-success-content',
      '--alex-color-status-success-surface',
      '--alex-color-status-success-border',
      '--alex-color-status-warning-surface',
      '--alex-color-status-danger-surface',
      '--alex-color-status-info-surface',
    ]));
    expect(WIDGET_APPEARANCE_VAR_NAMES).not.toEqual(
      expect.arrayContaining(RETIRED_WIDGET_VARIABLE_NAMES),
    );
    expect(WIDGET_APPEARANCE_VAR_NAMES.some(name => name.startsWith('--alex-appearance-token-')))
      .toBe(false);
  });

  it('passes canonical host overrides through unchanged', () => {
    const hostValues = {
      '--alex-color-action-primary-background': 'linear-gradient(test-primary)',
      '--alex-color-action-primary-content': '#101010',
      '--alex-color-action-primary-hover': 'linear-gradient(test-hover)',
      '--alex-color-action-primary-pressed': '#202020',
      '--alex-color-action-card-background': 'rgba(0, 0, 0, 0.07)',
      '--alex-color-composer-border': 'rgba(0, 0, 0, 0.14)',
      '--alex-color-composer-context-background': 'rgba(0, 0, 0, 0.06)',
      '--alex-color-content-caption': 'rgba(0, 0, 0, 0.45)',
      '--alex-component-button-primary-background': '#303030',
      '--alex-component-button-fill-background': 'rgba(0, 0, 0, 0.08)',
      '--alex-color-field-border-active': 'rgba(0, 0, 0, 0.20)',
      '--alex-color-field-group-background': 'rgba(0, 0, 0, 0.03)',
      '--alex-color-field-placeholder': 'rgba(0, 0, 0, 0.40)',
      '--alex-color-key-hint-content': 'rgba(0, 0, 0, 0.55)',
      '--alex-color-number-badge-background': 'rgba(0, 0, 0, 0.09)',
      '--alex-color-status-danger-surface': 'rgba(200, 0, 0, 0.12)',
      '--alex-color-status-danger-border': '#303030',
      '--alex-shadow-raised': '0 1px 2px #404040',
    };

    expect(readPayloadWithHostValues(hostValues)?.vars).toMatchObject(hostValues);
  });

  it('renders one self-contained fallback scope from canonical theme and system tokens', () => {
    const css = createWidgetAppearanceFallbackCss();

    for (const [name, value] of Object.entries(WIDGET_APPEARANCE_FALLBACK_VARS)) {
      expect(css).toContain(`      ${name}: ${value};`);
    }
    for (const [tokenName, value] of Object.entries(systemTokens)) {
      const variableName = systemCssVariables[tokenName as keyof typeof systemCssVariables];
      expect(css).toContain(`      ${variableName}: ${String(value)};`);
    }
    for (const retiredName of RETIRED_WIDGET_VARIABLE_NAMES) {
      expect(css).not.toContain(`${retiredName}:`);
    }
  });
});
