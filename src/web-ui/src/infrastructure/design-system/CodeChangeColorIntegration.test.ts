import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const CODE_CHANGE_TOKENS = [
  '--alex-color-code-change-added',
  '--alex-color-code-change-removed',
] as const;

const PRIMARY_CODE_CHANGE_SURFACES = [
  '../../flow_chat/components/modern/SessionFilesBadge.scss',
  '../../flow_chat/components/modern/SessionFileModificationsBar.scss',
  '../../flow_chat/components/usage/SessionUsageReportCard.scss',
  '../../flow_chat/components/InlineDiffPreview.scss',
  '../../flow_chat/tool-cards/SnapshotFullscreenDiffViewer.css',
  '../../tools/editor/components/DiffEditor.scss',
  '../../tools/git/components/BranchQuickSwitch.scss',
  '../../tools/git/components/GitDiffView/GitDiffView.scss',
  '../../tools/alex-canvas/runtime/sdk/data-display.tsx',
  '../../tools/alex-canvas/runtime/canvasRuntimeInstaller.ts',
  '../../tools/alex-canvas/runtime/styles/canvas-runtime.scss',
  '../../app/components/panels/review-platform/ReviewPlatformPanel.scss',
] as const;

describe('code-change color integration', () => {
  it('uses the shared addition and removal semantics across product surfaces', () => {
    for (const relativePath of PRIMARY_CODE_CHANGE_SURFACES) {
      const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8');

      for (const token of CODE_CHANGE_TOKENS) {
        expect(source, `${relativePath} should consume ${token}`).toContain(token);
      }
    }
  });

  it('keeps diff renderers independent from the appearance-specific Git palette', () => {
    for (const relativePath of [
      '../../flow_chat/components/InlineDiffPreview.scss',
      '../../tools/editor/components/DiffEditor.scss',
      '../../tools/git/components/GitDiffView/GitDiffView.scss',
    ]) {
      const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8');

      expect(source).not.toMatch(/--alex-domain-git-(?:added|deleted|staged)/);
    }
  });
});
