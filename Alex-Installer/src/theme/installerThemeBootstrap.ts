import '@alex/theme-alex/default.css';
import { themes } from '@alex/theme-alex';

const colorScheme = window.matchMedia?.('(prefers-color-scheme: dark)').matches
  ? 'dark'
  : 'light';
const root = document.documentElement;

root.setAttribute('data-alex-design-system-root', '');
root.setAttribute('data-color-scheme', colorScheme);
root.setAttribute('data-contrast', 'standard');
root.setAttribute('data-density', 'comfortable');
root.style.colorScheme = colorScheme;

document.querySelector('meta[name="theme-color"]')?.setAttribute(
  'content',
  String(themes[colorScheme]['color.surface.canvas']),
);
