import { describe, expect, it } from 'vitest';
import { ALEX_DOWNLOAD_URL, ALEX_HOME_URL } from './links';

describe('Skin Market external links', () => {
  it('uses the official Alex website and download pages', () => {
    expect(ALEX_HOME_URL).toBe('https://openbitfun.com/');
    expect(ALEX_DOWNLOAD_URL).toBe('https://openbitfun.com/download');
    expect(new URL(ALEX_HOME_URL).protocol).toBe('https:');
    expect(new URL(ALEX_DOWNLOAD_URL).protocol).toBe('https:');
  });
});
