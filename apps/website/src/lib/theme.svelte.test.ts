/* eslint-disable unicorn/no-document-cookie -- Verify that browser overrides preserve the saved theme. */

import { ThemeState } from '@typie/ui/context';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Theme } from '@typie/ui/context';

vi.mock('$app/state', () => ({ page: { url: new URL('https://typie.test/document') } }));

let destroy: (() => void) | undefined;
let browserForcesDark = false;

beforeEach(() => {
  browserForcesDark = false;
  const getComputedStyle = window.getComputedStyle;
  vi.spyOn(window, 'getComputedStyle').mockImplementation((element) => {
    const style = getComputedStyle(element);
    if (element instanceof HTMLElement && element.style.backgroundColor.toLowerCase() === 'canvas') {
      Object.defineProperty(style, 'backgroundColor', {
        value: browserForcesDark ? 'rgb(18, 18, 18)' : 'rgb(255, 255, 255)',
      });
    }
    return style;
  });
});

afterEach(() => {
  destroy?.();
  destroy = undefined;
  for (const name of ['typie-th', 'typie-th-lv', 'typie-th-dv']) {
    document.cookie = `${name}=; max-age=0; path=/`;
  }
  for (const name of ['data-theme', 'data-variant-light', 'data-variant-dark', 'data-no-transition']) {
    document.documentElement.removeAttribute(name);
  }
  vi.restoreAllMocks();
});

const start = async (mode: Theme) => {
  document.cookie = `typie-th=${mode}; path=/`;
  document.cookie = 'typie-th-dv=nord; path=/';
  let theme!: ThemeState;
  destroy = $effect.root(() => {
    theme = new ThemeState();
  });
  await tick();
  return theme;
};

it.each(['auto', 'light'] as const)('keeps the page and renderer dark when the browser overrides %s', async (mode) => {
  browserForcesDark = true;
  const theme = await start(mode);

  expect(theme.effectiveTheme).toBe('dark');
  expect(document.documentElement.dataset.theme).toBe('dark');
  expect(theme.currentThemeVariant).toBe('dark-nord');
  expect(theme.currentTheme).toBe(mode);
  expect(document.cookie).toContain(`typie-th=${mode}`);

  browserForcesDark = false;
  window.dispatchEvent(new Event('focus'));
  await tick();

  expect(theme.effectiveTheme).toBe('light');
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(theme.currentThemeVariant).toBe('light-white');
  expect(theme.currentTheme).toBe(mode);
});

it('rechecks browser darkening when returning to a page and preserves later theme choices', async () => {
  const theme = await start('light');
  expect(theme.effectiveTheme).toBe('light');

  browserForcesDark = true;
  document.dispatchEvent(new Event('visibilitychange'));
  await tick();
  expect(theme.effectiveTheme).toBe('dark');
  expect(document.documentElement.dataset.theme).toBe('dark');

  theme.currentTheme = 'dark';
  browserForcesDark = false;
  window.dispatchEvent(new Event('pageshow'));
  await tick();
  expect(theme.effectiveTheme).toBe('dark');
  expect(theme.currentTheme).toBe('dark');
});

it('does not interpret an accessibility color palette as automatic darkening', async () => {
  browserForcesDark = true;
  const matchMedia = window.matchMedia;
  vi.spyOn(window, 'matchMedia').mockImplementation((query) => ({
    ...matchMedia(query),
    matches: query === '(forced-colors: active)',
  }));
  const theme = await start('light');
  expect(theme.effectiveTheme).toBe('light');
  expect(document.documentElement.dataset.theme).toBe('light');
});
