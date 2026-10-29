// @vitest-environment jsdom

import React, { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { RevealableEntityBadge } from './RevealableEntityBadge';

const mocks = vi.hoisted(() => ({
  revealEntity: vi.fn(),
}));

vi.mock('@/infrastructure/api', () => ({
  basemindAPI: {
    revealEntity: (...args: unknown[]) => mocks.revealEntity(...args),
  },
}));

vi.mock('@/infrastructure/i18n', () => ({
  useI18n: () => ({
    t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
  }),
}));

vi.mock('@/shared/utils/logger', () => ({
  createLogger: () => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  }),
}));

describe('RevealableEntityBadge', () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    mocks.revealEntity.mockReset();
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    vi.clearAllMocks();
  });

  function render(): HTMLElement {
    act(() => {
      root.render(
        <RevealableEntityBadge entityType="PERSON" entityId="p1" maskedLabel="[PERSON_1]" workspaceId="workspace-1" />,
      );
    });
    const badge = container.querySelector('.pii-entity-badge');
    if (!badge) throw new Error('badge not rendered');
    return badge as HTMLElement;
  }

  it('reveals the original value for the owning workspace', async () => {
    mocks.revealEntity.mockResolvedValue('Ada Lovelace');
    const badge = render();
    expect(badge.textContent).toContain('[PERSON_1]');
    expect(badge.getAttribute('data-alex-state')).toBeNull();

    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    expect(mocks.revealEntity).toHaveBeenCalledWith('p1', 'workspace-1');
    expect(badge.textContent).toContain('Ada Lovelace');
    expect(badge.getAttribute('data-alex-state')).toBe('revealed');
    expect(badge.getAttribute('aria-pressed')).toBe('true');
  });

  it('blocks a second reveal while the first one is in flight', async () => {
    let resolveReveal: ((value: string) => void) | undefined;
    mocks.revealEntity.mockImplementation(() => new Promise<string>(resolve => {
      resolveReveal = resolve;
    }));
    const badge = render();

    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    expect(mocks.revealEntity).toHaveBeenCalledTimes(1);
    expect(badge.getAttribute('data-alex-state')).toBe('loading');

    await act(async () => {
      resolveReveal?.('Ada Lovelace');
    });
    expect(badge.getAttribute('data-alex-state')).toBe('revealed');
  });

  it('shows a visible error state when the reveal fails', async () => {
    mocks.revealEntity.mockRejectedValue(new Error('vault locked'));
    const badge = render();

    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    expect(badge.getAttribute('data-alex-state')).toBe('error');
    expect(badge.textContent).toContain('vault locked');
    expect(badge.textContent).toContain('[PERSON_1]');
  });

  it('hides a revealed value again on the next activation', async () => {
    mocks.revealEntity.mockResolvedValue('Ada Lovelace');
    const badge = render();

    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    expect(badge.textContent).toContain('Ada Lovelace');

    await act(async () => {
      badge.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    expect(badge.textContent).toContain('[PERSON_1]');
    expect(badge.getAttribute('data-alex-state')).toBeNull();
  });
});
