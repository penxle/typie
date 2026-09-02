import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cdp, userEvent } from 'vitest/browser';
import EntityMenu from './EntityMenu.svelte';
import type { CDPSession } from '@vitest/browser-playwright';

let component: Record<string, unknown> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const playwrightCdp = () => cdp() as CDPSession;
const openMenu = {
  pointer: async (button: HTMLButtonElement) => userEvent.click(button),
  keyboard: async (button: HTMLButtonElement) => {
    button.focus();
    await userEvent.keyboard('{ArrowDown}');
  },
};

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('entity tree menu trigger', () => {
  it('reclaims idle width and returns for hover, focus, and open state on a fine pointer', async () => {
    await playwrightCdp().send('Emulation.setTouchEmulationEnabled', { enabled: false });

    try {
      expect(matchMedia('(pointer: fine)').matches).toBe(true);

      const target = document.createElement('div');
      target.className = 'group';
      target.tabIndex = 0;
      Object.assign(target.style, { display: 'flex', width: '320px', height: '24px' });
      document.body.append(target);

      component = mount(EntityMenu, { target, props: { label: '엔티티 메뉴' } });
      await tick();

      const button = target.querySelector('button');
      expect(button).toBeInstanceOf(HTMLButtonElement);
      if (!(button instanceof HTMLButtonElement)) throw new Error('EntityMenu did not render its trigger button');

      expect(getComputedStyle(button).display).toBe('none');
      expect(button.getBoundingClientRect().width).toBe(0);

      await userEvent.hover(target);
      expect(getComputedStyle(button).display).toBe('flex');
      expect(button.getBoundingClientRect().width).toBe(16);

      await userEvent.unhover(target);
      expect(getComputedStyle(button).display).toBe('none');

      target.focus();
      expect(getComputedStyle(button).display).toBe('flex');

      await userEvent.hover(target);
      await userEvent.click(button);
      expect(button.getAttribute('aria-expanded')).toBe('true');

      document.body.tabIndex = -1;
      document.body.focus();
      await userEvent.unhover(target);
      expect(target.matches(':focus-within')).toBe(false);
      expect(getComputedStyle(button).display).toBe('flex');
    } finally {
      await playwrightCdp().send('Emulation.setTouchEmulationEnabled', { enabled: true });
    }
  });

  it.each(['pointer', 'keyboard'] as const)('keeps the closing menu anchored after $0 opening', async (opening) => {
    await playwrightCdp().send('Emulation.setTouchEmulationEnabled', { enabled: false });

    try {
      expect(matchMedia('(pointer: fine)').matches).toBe(true);

      const target = document.createElement('div');
      target.className = 'group';
      target.tabIndex = 0;
      Object.assign(target.style, {
        display: 'flex',
        position: 'fixed',
        left: '200px',
        top: '120px',
        width: '320px',
        height: '24px',
      });
      document.body.append(target);

      component = mount(EntityMenu, { target, props: { label: '엔티티 메뉴' } });
      await tick();

      const button = target.querySelector('button');
      expect(button).toBeInstanceOf(HTMLButtonElement);
      if (!(button instanceof HTMLButtonElement)) throw new Error('EntityMenu did not render its trigger button');

      await userEvent.hover(target);
      await openMenu[opening](button);
      await tick();
      await frame();

      const menu = document.querySelector('[role="menu"]');
      expect(menu).toBeInstanceOf(HTMLUListElement);
      if (!(menu instanceof HTMLUListElement)) throw new Error('EntityMenu did not render its menu');
      const floating = menu.parentElement;
      expect(floating).toBeInstanceOf(HTMLDivElement);
      if (!(floating instanceof HTMLDivElement)) throw new Error('EntityMenu did not render its floating wrapper');
      expect(Number.parseFloat(floating.style.left)).toBeGreaterThan(100);
      expect(Number.parseFloat(floating.style.top)).toBeGreaterThan(100);

      document.body.tabIndex = -1;
      document.body.focus();
      await userEvent.unhover(target);
      await userEvent.keyboard('{Escape}');
      document.body.focus();
      await tick();
      await frame();

      expect(button.getAttribute('aria-expanded')).toBe('false');
      expect(menu.isConnected).toBe(true);
      expect(floating.isConnected).toBe(true);
      expect(getComputedStyle(button).display).toBe('flex');
      expect(button.getBoundingClientRect().width).toBe(16);
      expect(Number.parseFloat(floating.style.left)).toBeGreaterThan(100);
      expect(Number.parseFloat(floating.style.top)).toBeGreaterThan(100);

      await vi.waitFor(() => expect(menu.isConnected).toBe(false), { timeout: 1000 });
      await tick();
      expect(getComputedStyle(button).display).toBe('none');
      expect(button.getBoundingClientRect().width).toBe(0);
    } finally {
      await playwrightCdp().send('Emulation.setTouchEmulationEnabled', { enabled: true });
    }
  });

  it('stays visible and accessible in the browser test coarse-pointer environment', async () => {
    const target = document.createElement('div');
    target.className = 'group';
    target.tabIndex = 0;
    Object.assign(target.style, { display: 'flex', width: '320px' });
    document.body.append(target);

    component = mount(EntityMenu, { target, props: { label: '엔티티 메뉴' } });
    await tick();

    const button = target.querySelector('button');
    expect(button).toBeInstanceOf(HTMLButtonElement);
    if (!(button instanceof HTMLButtonElement)) throw new Error('EntityMenu did not render its trigger button');

    expect(matchMedia('(pointer: coarse)').matches).toBe(true);
    expect(getComputedStyle(button).display).not.toBe('none');
    expect(button.getBoundingClientRect().width).toBe(16);
    expect(button.getAttribute('aria-label')).toBe('엔티티 메뉴');
    const pressedMarker = button.querySelector('[aria-pressed]');
    expect(pressedMarker?.getAttribute('aria-pressed')).toBe('false');
    expect(pressedMarker?.getBoundingClientRect().width).toBe(16);
    expect(pressedMarker?.getBoundingClientRect().height).toBe(16);

    target.focus();
    await frame();

    await userEvent.keyboard('{Tab}');
    expect(document.activeElement).toBe(button);
    expect(button.matches(':focus-visible')).toBe(true);
    expect(getComputedStyle(button).backgroundColor).not.toBe('rgba(0, 0, 0, 0)');

    await userEvent.click(button);
    expect(button.getAttribute('aria-expanded')).toBe('true');
    expect(button.querySelector('[aria-pressed]')?.getAttribute('aria-pressed')).toBe('true');
    expect(getComputedStyle(button).display).not.toBe('none');
  });
});
