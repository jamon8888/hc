import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

import { buildCanvasRuntimeInstallerScript } from './canvasRuntimeInstaller';

describe('Canvas runtime installer', () => {
  it('loads generated typography tokens while keeping iframe-local shape and spacing fallbacks', () => {
    const runtimeCss = readFileSync(new URL('./styles/canvas-runtime.scss', import.meta.url), 'utf8');
    const runtimeEntry = readFileSync(new URL('./entry.tsx', import.meta.url), 'utf8');
    const iframeFallbackVars = [
      '--alex-radius-sm',
      '--alex-radius-base',
      '--alex-radius-md',
      '--alex-radius-lg',
      '--alex-radius-xl',
      '--alex-radius-2xl',
      '--alex-radius-pill',
      '--alex-space-1',
      '--alex-space-2',
      '--alex-space-3',
      '--alex-space-4',
      '--alex-space-5',
      '--alex-space-6',
      '--alex-space-8',
      '--alex-space-10',
      '--alex-space-12',
      '--alex-space-16',
    ];

    for (const name of iframeFallbackVars) {
      expect(runtimeCss).toContain(`${name}:`);
    }
    expect(runtimeEntry).toContain("import '@alex/design-tokens/tokens.css';");
    expect(runtimeCss).not.toMatch(/^\s*--alex-(?:font|letter-spacing|line-height|type)-/m);
  });

  it('merges bundled SDK adapters before user module startup', () => {
    const script = buildCanvasRuntimeInstallerScript('rev_test');

    expect(script).toContain('function installSdkAdapters()');
    expect(script).toContain('...runtimeWindow.AlexCanvasSDKAdapters');
    expect(script).toContain('runtimeWindow.AlexCanvasRuntimeHooks');
    expect(script.indexOf('installSdkAdapters();')).toBeLessThan(
      script.indexOf('alex-canvas-module-started'),
    );
  });

  it('keeps fallback SDK scoped to runtime hooks while bundled adapters own components', () => {
    const script = buildCanvasRuntimeInstallerScript('rev_test');

    expect(script).toContain('runtimeWindow.AlexCanvasSDK = {');
    expect(script).toContain('...runtimeWindow.AlexCanvasRuntimeHooks');
    expect(script).not.toContain('function Stack');
    expect(script).not.toContain('function BarChart');
    expect(script).not.toContain('function DependencyGraph');
  });

  it('syncs browser color scheme when the host appearance changes', () => {
    const script = buildCanvasRuntimeInstallerScript('rev_test');

    expect(script).toContain('nextAppearance.type === "dark" || nextAppearance.type === "light"');
    expect(script).toContain('document.documentElement.style.colorScheme = nextAppearance.type');
  });

  it('projects code-change colors into the renderer without changing status colors', () => {
    const runtimeCss = readFileSync(new URL('./styles/canvas-runtime.scss', import.meta.url), 'utf8');
    const script = buildCanvasRuntimeInstallerScript('rev_test');

    expect(runtimeCss).toContain('--alex-color-code-change-added: var(--alex-canvas-success)');
    expect(runtimeCss).toContain('--alex-color-code-change-removed: var(--alex-canvas-danger)');
    expect(script).toContain('appearanceVars["--alex-color-code-change-added"]');
    expect(script).toContain('appearanceVars["--alex-color-code-change-removed"]');
    expect(script).toContain('stripAdded: codeChangeAdded');
    expect(script).toContain('stripRemoved: codeChangeRemoved');
  });

  it('installs design-mode element selection handlers', () => {
    const script = buildCanvasRuntimeInstallerScript('rev_test');

    expect(script).toContain('alex-canvas-design-mode');
    expect(script).toContain('data-alex-canvas-design-mode');
    expect(script).toContain('alex-canvas-element-selected');
    expect(script).toContain('document.addEventListener("pointermove"');
    expect(script).toContain('document.addEventListener("click"');
  });
});
