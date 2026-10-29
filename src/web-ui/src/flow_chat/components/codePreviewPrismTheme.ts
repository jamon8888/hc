/**
 * Prism themes for Flow Chat embedded code previews.
 */
import type { CSSProperties } from 'react';
import { buildSharedPrismStyle } from '@/shared/prism/prismTheme';

export function buildCodePreviewPrismStyle(isLight: boolean): Record<string, CSSProperties> {
  return buildSharedPrismStyle(isLight, {
    pre: {
      margin: 0,
      padding: 0,
      fontSize: 'var(--alex-type-flow-code-font-size)',
      lineHeight: 'var(--alex-type-flow-code-line-height)',
      fontFamily: 'var(--alex-type-flow-code-font-family)',
      fontWeight: 'var(--alex-type-flow-code-font-weight)',
    },
    code: {
      fontSize: 'var(--alex-type-flow-code-font-size)',
      lineHeight: 'var(--alex-type-flow-code-line-height)',
      fontFamily: 'var(--alex-type-flow-code-font-family)',
      fontWeight: 'var(--alex-type-flow-code-font-weight)',
    },
  });
}
