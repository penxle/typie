/* eslint-disable unicorn/no-document-cookie -- Verify persisted cookies without mocking the theme context. */

import { Dialog } from '@typie/ui/notification';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import ThemeTabTestRoot from './theme-tab-test-root.svelte';

vi.mock('$app/state', () => ({ page: { url: new URL('https://typie.test/dashboard') } }));
vi.mock('mixpanel-browser', () => ({ default: { track: vi.fn() } }));

let component: Record<string, unknown> | undefined;
let confirmation: Parameters<typeof Dialog.confirm>[0] | undefined;

beforeEach(() => {
  confirmation = undefined;
  vi.spyOn(Dialog, 'confirm').mockImplementation((options) => {
    confirmation = options;
  });
});

afterEach(async () => {
  if (component) await unmount(component);
  confirmation?.onclose?.();
  component = undefined;
  document.body.replaceChildren();
  for (const name of ['typie-th', 'typie-th-lv', 'typie-th-dv']) {
    document.cookie = `${name}=; max-age=0; path=/`;
  }
  for (const name of ['data-theme', 'data-variant-light', 'data-variant-dark', 'data-no-transition']) {
    document.documentElement.removeAttribute(name);
  }
  vi.restoreAllMocks();
});

const start = async (mode: string, systemDark = false) => {
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  vi.spyOn(window, 'matchMedia').mockReturnValue({ ...media, matches: systemDark });
  document.cookie = `typie-th=${mode}; path=/`;
  document.cookie = 'typie-th-lv=everforest; path=/';
  document.cookie = 'typie-th-dv=nord; path=/';
  component = mount(ThemeTabTestRoot, { target: document.body });
  await tick();
};

const click = async (label: string) => {
  const button = [...document.querySelectorAll<HTMLButtonElement>('button')].find(
    (candidate) => (candidate.getAttribute('aria-label') ?? candidate.textContent?.trim()) === label,
  );
  if (!button) throw new Error(`Missing button: ${label}`);
  button.click();
  await tick();
};

const displayedMode = () => document.querySelector('[data-testid="displayed-mode"]')?.textContent;

const selectedMode = () => {
  const button = document.querySelector('[aria-label="화면 모드"] [aria-selected="true"], [aria-label="화면 모드"] [aria-checked="true"]');
  return button?.getAttribute('aria-label') ?? button?.textContent?.trim();
};

it.each([
  { initial: 'light', modeLabel: '라이트 모드', paletteLabel: 'Monokai' },
  { initial: 'dark', modeLabel: '다크 모드', paletteLabel: 'GitHub Light' },
  { initial: 'auto', modeLabel: '시스템 설정', paletteLabel: 'Monokai' },
])('keeps $modeLabel selected while previewing a palette', async ({ initial, modeLabel, paletteLabel }) => {
  await start(initial);
  expect(selectedMode()).toBe(modeLabel);
  await click(paletteLabel);
  expect(selectedMode()).toBe(modeLabel);
});

it.each([
  { initial: 'dark', preview: 'light', label: 'GitHub Light', variantCookie: 'typie-th-lv=github', otherCookie: 'typie-th-dv=nord' },
  { initial: 'light', preview: 'dark', label: 'Monokai', variantCookie: 'typie-th-dv=monokai', otherCookie: 'typie-th-lv=everforest' },
])(
  'previews $preview without changing $initial mode, then persists the confirmed mode',
  async ({ initial, preview, label, variantCookie, otherCookie }) => {
    await start(initial);
    await click(label);
    expect(displayedMode()).toBe(preview);
    expect(document.cookie).toContain(`typie-th=${initial}`);
    expect(document.cookie).toContain(variantCookie);
    expect(document.cookie).toContain(otherCookie);
    expect(confirmation).toBeUndefined();

    await click('테마 나가기');
    expect(confirmation).toBeDefined();
    expect(displayedMode()).toBe(preview);
    await confirmation?.actionHandler?.();
    confirmation?.onclose?.();
    await tick();
    expect(document.cookie).toContain(`typie-th=${preview}`);
    expect(displayedMode()).toBe(preview);

    confirmation = undefined;
    await click('테마 열기');
    await click('테마 나가기');
    expect(confirmation).toBeUndefined();
    expect(document.cookie).toContain(`typie-th=${preview}`);
  },
);

it.each([
  { initial: 'light', systemDark: false, label: 'Monokai', expected: 'light', returnTo: '라이트 모드로' },
  { initial: 'dark', systemDark: false, label: 'GitHub Light', expected: 'dark', returnTo: '다크 모드로' },
  { initial: 'auto', systemDark: false, label: 'Monokai', expected: 'light', returnTo: '시스템 설정으로' },
  { initial: 'auto', systemDark: true, label: 'GitHub Light', expected: 'dark', returnTo: '시스템 설정으로' },
])(
  'restores $initial (system dark: $systemDark) when declining the preview mode',
  async ({ initial, systemDark, label, expected, returnTo }) => {
    await start(initial, systemDark);
    await click(label);
    await click('테마 나가기');
    expect(confirmation).toBeDefined();
    expect(confirmation?.message).toContain(`${returnTo} 돌아갈 수 있어요`);
    confirmation?.cancelHandler?.();
    confirmation?.onclose?.();
    await tick();
    expect(displayedMode()).toBe(expected);
    expect(document.cookie).toContain(`typie-th=${initial}`);
  },
);

it.each([
  { initial: 'light', systemDark: false, label: 'GitHub Light' },
  { initial: 'dark', systemDark: false, label: 'Monokai' },
  { initial: 'auto', systemDark: false, label: 'GitHub Light' },
  { initial: 'auto', systemDark: true, label: 'Monokai' },
])('does not ask when the preview matches $initial (system dark: $systemDark)', async ({ initial, systemDark, label }) => {
  await start(initial, systemDark);
  await click(label);
  await click('테마 나가기');
  expect(confirmation).toBeUndefined();
  expect(document.cookie).toContain(`typie-th=${initial}`);
});

it.each([
  { label: '라이트 모드', expected: 'light', displayed: 'light' },
  { label: '다크 모드', expected: 'dark', displayed: 'dark' },
  { label: '시스템 설정', expected: 'auto', displayed: 'light' },
])('applies an explicit $label choice immediately without asking on exit', async ({ label, expected, displayed }) => {
  await start('light');
  await click('Monokai');
  await click(label);
  expect(displayedMode()).toBe(displayed);
  expect(selectedMode()).toBe(label);
  expect(document.cookie).toContain(`typie-th=${expected}`);
  await click('테마 나가기');
  expect(confirmation).toBeUndefined();
});
