import '../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import MenuOverflowTestRoot from './menu-overflow-test-root.svelte';

vi.mock('$app/navigation', () => ({ afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn(), tryAppContext: vi.fn() }));

const VIEWPORT_WIDTH = 800;
const VIEWPORT_HEIGHT = 400;
const TOLERANCE_PX = 1;

let component: ReturnType<typeof mount> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const element = (testId: string): HTMLElement => {
  const found = document.querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  if (!found) throw new Error(`Missing element: ${testId}`);
  return found;
};

const openMenu = async (): Promise<HTMLElement> => {
  let menu: HTMLElement | null = null;
  await vi.waitFor(() => {
    menu = document.querySelector<HTMLElement>('[role="menu"]');
    expect(menu).not.toBeNull();
  });
  if (!menu) throw new Error('Expected a menu');
  const menuElement: HTMLElement = menu;
  await vi.waitFor(() => {
    expect(menuElement.getAnimations({ subtree: true })).toHaveLength(0);
  });
  await frame();
  return menuElement;
};

const expectWithinViewport = (menu: HTMLElement, context: string) => {
  const rect = menu.getBoundingClientRect();
  const summary = `${context}: menu=${rect.top}..${rect.bottom} viewport=0..${window.innerHeight}`;
  expect(rect.top, summary).toBeGreaterThanOrEqual(-TOLERANCE_PX);
  expect(rect.bottom, summary).toBeLessThanOrEqual(window.innerHeight + TOLERANCE_PX);
};

const expectDocumentUnchanged = (context: string) => {
  const summary = `${context}: scrollY=${window.scrollY} scrollHeight=${document.documentElement.scrollHeight} innerHeight=${window.innerHeight}`;
  expect(window.scrollY, summary).toBe(0);
  expect(document.documentElement.scrollHeight, summary).toBeLessThanOrEqual(window.innerHeight);
};

const expectScrolledToTop = (menu: HTMLElement, context: string) => {
  const summary = `${context}: scrollTop=${menu.scrollTop} clientHeight=${menu.clientHeight} scrollHeight=${menu.scrollHeight} active=${document.activeElement?.outerHTML.slice(0, 80)}`;
  expect(menu.scrollHeight, summary).toBeGreaterThan(menu.clientHeight);
  expect(menu.scrollTop, summary).toBe(0);
};

const focusLastItem = async (menu: HTMLElement) => {
  const items = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  const last = items.at(-1);
  if (!last) throw new Error('Expected menu items');
  last.focus();
  await frame();
  await frame();
};

const mountFixture = async (props: { itemCount?: number; anchorTop?: number; anchorLeft?: number } = {}) => {
  component = mount(MenuOverflowTestRoot, { target: document.body, props });
  await tick();
  await frame();
};

beforeEach(async () => {
  await page.viewport(VIEWPORT_WIDTH, VIEWPORT_HEIGHT);
  window.scrollTo(0, 0);
});

afterEach(() => {
  if (component) {
    unmount(component);
    component = undefined;
  }
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe('menu overflow in a small viewport', () => {
  it('keeps a button menu inside the viewport and scrolls its items instead of the document', async () => {
    await mountFixture({ anchorTop: 200 });

    element('menu-trigger').click();
    const menu = await openMenu();

    expectWithinViewport(menu, 'button menu');
    expectDocumentUnchanged('button menu opened');
    expectScrolledToTop(menu, 'button menu opened');

    await focusLastItem(menu);
    expectWithinViewport(menu, 'button menu after focusing last item');
    expectDocumentUnchanged('button menu after focusing last item');
  });

  it('keeps a select dropdown inside the viewport', async () => {
    await mountFixture({ anchorTop: 200 });

    element('select-trigger').querySelector('button')?.click();
    const menu = await openMenu();

    expectWithinViewport(menu, 'select');
    expectDocumentUnchanged('select opened');

    await focusLastItem(menu);
    expectDocumentUnchanged('select after focusing last item');
  });

  it('keeps a context menu inside the viewport when opened near the bottom edge', async () => {
    await mountFixture({ anchorTop: 300 });

    const target = element('context-target');
    const rect = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: rect.left + 10, clientY: rect.top + 10 }),
    );
    const menu = await openMenu();

    expectWithinViewport(menu, 'context menu');
    expectDocumentUnchanged('context menu opened');
    expectScrolledToTop(menu, 'context menu opened');

    await focusLastItem(menu);
    expectWithinViewport(menu, 'context menu after focusing last item');
    expectDocumentUnchanged('context menu after focusing last item');
  });

  it('keeps a context menu inside the viewport when opened near the top edge', async () => {
    await mountFixture({ anchorTop: 20 });

    const target = element('context-target');
    const rect = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: rect.left + 10, clientY: rect.top + 10 }),
    );
    const menu = await openMenu();

    expectWithinViewport(menu, 'context menu near top');
    expectDocumentUnchanged('context menu near top');
    expectScrolledToTop(menu, 'context menu near top');
  });

  it('keeps a submenu inside the viewport when its trigger sits near the bottom edge', async () => {
    await mountFixture({ anchorTop: 300 });

    const target = element('context-target');
    const rect = target.getBoundingClientRect();
    target.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: rect.left + 10, clientY: rect.top + 10 }),
    );
    const menu = await openMenu();

    const trigger = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((item) => item.textContent?.trim() === 'Submenu');
    if (!trigger) throw new Error('Missing submenu trigger');
    trigger.scrollIntoView({ block: 'nearest' });
    trigger.dispatchEvent(new PointerEvent('pointerenter'));

    let submenu: HTMLElement | undefined;
    await vi.waitFor(() => {
      const menus = document.querySelectorAll<HTMLElement>('[role="menu"]');
      expect(menus).toHaveLength(2);
      submenu = menus[1];
    });
    if (!submenu) throw new Error('Expected a submenu');
    await vi.waitFor(() => {
      expect(submenu?.getAnimations({ subtree: true })).toHaveLength(0);
    });
    await frame();

    expectWithinViewport(submenu, 'submenu');
    expectDocumentUnchanged('submenu opened');
  });
});
