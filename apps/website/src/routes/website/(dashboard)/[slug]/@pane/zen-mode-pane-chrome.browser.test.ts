import '../../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ZenModePaneChromeTestRoot from './zen-mode-pane-chrome-test-root.svelte';

let component: ReturnType<typeof mount> | undefined;

const move = (target: HTMLElement, x: number, y: number, pointerType = 'mouse') => {
  target.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: x, clientY: y, pointerType }));
};

beforeEach(() => {
  vi.useFakeTimers();
  const noop = () => null;
  vi.spyOn(window, 'matchMedia').mockImplementation(
    (query) =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: noop,
        removeEventListener: noop,
        addListener: noop,
        removeListener: noop,
        dispatchEvent: () => true,
      }) satisfies MediaQueryList,
  );
});

afterEach(async () => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('pane focus-mode chrome', () => {
  it('reveals only the directly intended header segment', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-top]')?.value).toBe('78');

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(500);
    await tick();

    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('37');
  });

  it('keeps a revealed header foreground draggable while transparent chrome passes through', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();
    const clientX = rect.left + 40;
    const clientY = rect.top + 18;

    move(root, clientX, clientY);
    vi.advanceTimersByTime(500);
    await tick();

    const hit = document.elementFromPoint(clientX, clientY) as HTMLElement | null;
    expect(hit?.closest('[data-chrome-identity]')).not.toBeNull();
    expect(getComputedStyle(hit as HTMLElement).cursor).toBe('grab');

    hit?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, isPrimary: true }));
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-pointer-down-count]')?.value).toBe('1');
  });

  it('hands the revealed header lane foreground to pane drag while the expansion is still spreading', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();
    const clientX = rect.left + 300;
    const clientY = rect.top + 18;

    move(root, clientX, clientY);
    vi.advanceTimersByTime(999);
    await tick();
    expect(
      (document.elementFromPoint(clientX, clientY) as HTMLElement | null)?.closest(
        '[data-chrome-header-foreground-hit], [data-chrome-identity], [data-chrome-actions]',
      ),
    ).toBeNull();

    vi.advanceTimersByTime(1);
    await tick();
    const hit = document.querySelector<HTMLElement>('[data-chrome-header-foreground-hit]');
    expect(hit?.style.pointerEvents).toBe('auto');
    expect(hit?.style.clipPath).toContain('circle(');
    expect(getComputedStyle(hit as HTMLElement).cursor).toBe('grab');

    hit?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, isPrimary: true }));
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-pointer-down-count]')?.value).toBe('1');
  });

  it('starts hidden and applies normal hover intent to the segment under the pointer after activation', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body, props: { initialActive: false } });
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-top]')?.value).toBe('78');

    (component as typeof component & { setActive(next: boolean): void }).setActive(true);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-top]')?.value).toBe('78');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('idle');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');

    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();
    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(399);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('pending');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
  });

  it('enters with the chrome under the existing pointer already revealed', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body, props: { initialActive: false } });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    (component as typeof component & { setActive(next: boolean): void }).setActive(true);
    await tick();

    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('78');
  });

  it('keeps the clicked actions segment revealed across repeated entry when layout reclassifies the pointer', async () => {
    component = mount(ZenModePaneChromeTestRoot, {
      target: document.body,
      props: { initialActive: false, reclassifyActionsOnEntry: true },
    });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toggle = document.querySelector<HTMLButtonElement>('[data-chrome-focus-toggle]');
    if (!root || !toggle) throw new Error('Missing pane chrome fixture');
    const toggleRect = toggle.getBoundingClientRect();

    move(root, toggleRect.left + toggleRect.width / 2, toggleRect.top + toggleRect.height / 2);
    toggle.click();
    await tick();
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');

    toggle.click();
    await tick();
    toggle.click();
    await tick();
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
  });

  it('places toolbar effects and content after the shared header boundary', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();

    const header = document.querySelector<HTMLElement>('[data-chrome-header]');
    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar-content]');
    if (!header || !toolbarEffects || !toolbar) throw new Error('Missing pane chrome fixture');

    expect(toolbarEffects.getBoundingClientRect().top).toBe(header.getBoundingClientRect().bottom);
    expect(toolbar.getBoundingClientRect().top).toBe(header.getBoundingClientRect().bottom);
  });

  it('drops a stale segment hover hold when layout changes beneath the pointer', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    (component as typeof component & { holdActions(): void }).holdActions();
    await tick();
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-surface="actions"]')?.style.opacity).toBe('1');

    root.style.width = '720px';
    await vi.advanceTimersByTimeAsync(16);
    move(root, rect.left + 360, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();

    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-surface="actions"]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]')?.dataset.zenPaneChromeSpotX).toBe('360');
  });

  it('promotes toolbar intent and clears the group with one grace and fade', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(400);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-surface]')?.style.maskImage).toContain('radial-gradient');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.inert).toBe(true);

    vi.advanceTimersByTime(500);
    await tick();
    const toolbarForeground = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-surface]')?.style.maskImage).toContain('radial-gradient');
    expect(toolbarForeground?.style.opacity).toBe('1');
    expect(toolbarForeground?.inert).toBe(true);
    expect(toolbarForeground?.style.maskImage).toContain('radial-gradient');
    expect(toolbarForeground?.style.getPropertyValue('--zen-pane-chrome-foreground-opacity')).toBe('0');
    expect(toolbarForeground?.style.getPropertyValue('--zen-pane-chrome-foreground-radius')).toBe('88px');
    expect(toolbarEffects?.style.getPropertyValue('--zen-pane-chrome-spot-radius')).not.toBe('88px');

    vi.advanceTimersByTime(100);
    await tick();
    expect(toolbarForeground?.inert).toBe(false);
    expect(toolbarForeground?.style.getPropertyValue('--zen-pane-chrome-foreground-opacity')).toBe('1');
    expect(toolbarForeground?.style.getPropertyValue('--zen-pane-chrome-foreground-radius')).not.toBe('88px');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('1');
    expect(
      document
        .querySelector<HTMLElement>('[data-chrome-toolbar]')
        ?.contains(document.querySelector('[data-zen-pane-chrome-toolbar-surface]')),
    ).toBe(false);
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-surface]')).toBeNull();
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-spot]')?.style.maskImage).toContain(
      'radial-gradient',
    );
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('78');

    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(1000);
    move(root, rect.left + 320, rect.top + 150);
    vi.advanceTimersByTime(500);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('fading');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('0');

    vi.advanceTimersByTime(400);
    await tick();
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('0');
  });

  it('extends the exit grace after interacting with revealed chrome', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    if (!root || !toolbar) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    toolbar.click();
    move(root, rect.left + 300, rect.top + 140);

    vi.advanceTimersByTime(2999);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('fading');
  });

  it('refreshes warm state when an open menu closes', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();

    (component as typeof component & { openActionsMenu(): void }).openActionsMenu();
    await tick();
    await vi.advanceTimersByTimeAsync(0);
    vi.advanceTimersByTime(20_000);

    (component as typeof component & { closeActionsMenu(): void }).closeActionsMenu();
    await tick();
    await vi.advanceTimersByTimeAsync(0);
    vi.advanceTimersByTime(2999);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('fading');
  });

  it('uses the accelerated reveal pace when recently used chrome is revealed again', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    if (!root || !toolbar) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    toolbar.click();
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(3400);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('idle');

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(149);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('pending');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');

    vi.advanceTimersByTime(16);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
  });

  it('keeps the warm reveal pace when the wall clock changes', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    if (!root || !toolbar) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    toolbar.click();
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(3400);
    vi.setSystemTime(new Date(Date.now() + 60_000));

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(149);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('pending');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
  });

  it('refreshes warm state when a focused chrome action is activated', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const action = document.querySelector<HTMLButtonElement>('[data-chrome-toolbar-action]');
    if (!root || !action) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    await tick();
    action.focus();
    await tick();
    expect(document.activeElement).toBe(action);
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(20_000);

    action.click();
    action.blur();
    vi.advanceTimersByTime(2999);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('fading');
  });

  it('restores the cold reveal pace when the warm window expires', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    if (!root || !toolbar) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    toolbar.click();
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(20_000);

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(399);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('pending');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
  });

  it('restores the cold reveal pace after focus mode is re-entered', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbar = document.querySelector<HTMLElement>('[data-chrome-toolbar]');
    if (!root || !toolbar) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1000);
    toolbar.click();
    move(root, rect.left + 300, rect.top + 140);
    (component as typeof component & { setActive(next: boolean): void }).setActive(false);
    await tick();
    (component as typeof component & { setActive(next: boolean): void }).setActive(true);
    await tick();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(399);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('pending');

    vi.advanceTimersByTime(1);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
  });

  it('makes the zoom hover target available as toolbar expansion starts', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-inset]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-attached]')?.value).toBe('true');

    vi.advanceTimersByTime(180);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-inset]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('37');

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(99);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('preview');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');

    vi.advanceTimersByTime(1);
    await tick();

    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
    expect(toolbarEffects?.style.getPropertyValue('--zen-pane-chrome-spot-radius')).toBe('88px');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-surface]')?.style.maskImage).toContain('radial-gradient');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('37');

    vi.advanceTimersByTime(16);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(toolbarEffects?.style.getPropertyValue('--zen-pane-chrome-spot-radius')).not.toBe('88px');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('78');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-ready]')?.value).toBe('true');
  });

  it('hit-tests only the toolbar foreground region as it reveals', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const editor = document.querySelector<HTMLElement>('[data-editor-target]');
    if (!root || !editor) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();
    const originX = rect.left + 300;
    const originY = rect.top + 56;

    move(root, originX, originY);
    vi.advanceTimersByTime(999);
    await tick();

    expect(document.elementFromPoint(originX, originY)).toBe(editor);

    vi.advanceTimersByTime(1);
    await tick();

    expect(document.elementFromPoint(originX, originY)?.closest('[data-chrome-toolbar]')).not.toBeNull();
    expect(document.elementFromPoint(rect.left + 20, originY)).toBe(editor);
  });

  it('attaches the zoom surface directly to a revealed header', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(580);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-inset]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-inset]')?.value).toBe('37');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-zoom-attached]')?.value).toBe('true');
  });

  it('keeps a revealed header visible while its attached zoom control is hovered', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(580);
    await tick();

    (component as typeof component & { holdChromeAttachment(): void }).holdChromeAttachment();
    vi.advanceTimersByTime(2000);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).not.toBe('fading');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
  });

  it('lets an attachment hold chrome revealed later without revealing chrome itself', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    (component as typeof component & { holdChromeAttachment(): void }).holdChromeAttachment();
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('idle');

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(580);
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(2000);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
  });

  it('does not reveal the toolbar when attached zoom takes hover ownership', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(580);
    await tick();

    move(root, rect.right - 40, rect.top + 54);
    (component as typeof component & { holdChromeAttachment(): void }).holdChromeAttachment();
    vi.advanceTimersByTime(500);
    await tick();

    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('37');
  });

  it('does not project an attached zoom hover into either chrome lane', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(580);
    await tick();

    (component as typeof component & { holdChromeAttachmentAt(clientX: number, clientY: number): void }).holdChromeAttachmentAt(
      rect.right - 40,
      rect.top + 54,
    );
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-pointer]')?.value).toBe('null');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-toolbar-pointer]')?.value).toBe('null');
  });

  it('publishes the pointer only to the lane under it', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1900);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-pointer]')?.value).toBe('null');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-toolbar-pointer]')?.value).not.toBe('null');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-spot]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-spot]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-edge]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-edge]')?.style.opacity).toBe('1');

    move(root, rect.left + 40, rect.top + 18);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-header-pointer]')?.value).not.toBe('null');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-toolbar-pointer]')?.value).toBe('null');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-spot]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-spot]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-edge]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-edge]')?.style.opacity).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-edge]')?.dataset.zenPaneChromeEngagedEdgeRadius).toBe(
      '80',
    );
  });

  it('lights both toolbar boundaries around the pointer', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    if (!root || !toolbarEffects) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1900);
    await tick();

    const topEdge = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="top"]');
    const bottomEdge = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="bottom"]');
    expect(topEdge?.getBoundingClientRect().top).toBe(toolbarEffects.getBoundingClientRect().top - 1);
    expect(bottomEdge?.getBoundingClientRect().bottom).toBe(toolbarEffects.getBoundingClientRect().bottom);
  });

  it('keeps the boundary between two toolbar rows and lights it around the pointer', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body, props: { toolbarRows: 2 } });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    if (!root || !toolbarEffects) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1900);
    await tick();

    const transientSeparator = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-edge-boundary="separator"]');
    const engagedSeparator = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="separator"]');
    expect(transientSeparator?.getBoundingClientRect().top).toBe(toolbarEffects.getBoundingClientRect().top + 40);
    expect(engagedSeparator?.getBoundingClientRect().top).toBe(toolbarEffects.getBoundingClientRect().top + 40);
  });

  it('confines the pointer light and its boundaries to the active toolbar row', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body, props: { toolbarRows: 2 } });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const engagedSpot = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-engaged-spot]');
    if (!root || !engagedSpot) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(1900);
    await tick();

    const firstRowClip = engagedSpot.style.clipPath;
    expect(firstRowClip).not.toBe('');
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="top"]')).not.toBeNull();
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="separator"]')).not.toBeNull();
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="bottom"]')).toBeNull();

    move(root, rect.left + 300, rect.top + 100);
    await tick();

    expect(engagedSpot.style.clipPath).not.toBe(firstRowClip);
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="top"]')).toBeNull();
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="separator"]')).not.toBeNull();
    expect(document.querySelector('[data-zen-pane-chrome-toolbar-engaged-edge-boundary="bottom"]')).not.toBeNull();
  });

  it('keeps the transient segment surface underneath its engaged hover fade', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(1400);
    await tick();

    const laneSurface = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]');
    expect(laneSurface?.style.maskImage).not.toBe('linear-gradient(transparent, transparent)');
  });

  it('leaves a fading engaged spot at its last pointer position when the pointer changes lanes', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const engagedSpot = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-spot]');
    if (!root || !engagedSpot) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(1400);
    await tick();
    const lastPointerMask = engagedSpot.style.maskImage;

    move(root, rect.left + 300, rect.top + 56);
    await tick();

    expect(engagedSpot.style.opacity).toBe('0');
    expect(engagedSpot.style.maskImage).toBe(lastPointerMask);
  });

  it('shows a middle-gap spot before expanding both header segments', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]')?.value).toBe('0');
    expect(document.querySelector<HTMLElement>('[data-chrome-header-foreground-hit]')?.style.pointerEvents).toBe('none');
    expect(document.querySelector('[data-zen-pane-chrome-header-lane-surface]')).not.toBeNull();
    expect(document.querySelectorAll('[data-zen-pane-chrome-header-surface]')).toHaveLength(2);

    move(root, rect.left + 340, rect.top + 18);
    await tick();
    const laneSurface = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]');
    expect(laneSurface?.dataset.zenPaneChromeSpotX).toBe('340');
    expect(laneSurface?.dataset.zenPaneChromeSpotStrength).toBe('0.7');

    vi.advanceTimersByTime(600);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]')?.style.maskImage).toContain('radial-gradient');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.maskImage).toContain('radial-gradient');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
  });

  it('shares cold hover intent while the pointer moves between the header and toolbar lanes', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(250);
    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(150);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-surface]')?.style.maskImage).toContain('radial-gradient');

    vi.advanceTimersByTime(250);
    move(root, rect.left + 340, rect.top + 18);
    vi.advanceTimersByTime(250);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-toolbar]')?.style.opacity).toBe('0');
  });

  it('publishes one chrome inset as each revealed lane joins the layout', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();
    const inset = document.querySelector<HTMLOutputElement>('[data-chrome-header-inset]');
    const topInset = document.querySelector<HTMLOutputElement>('[data-chrome-occlusion]');

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(900);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(inset?.value).toBe('37');
    expect(topInset?.value).toBe('78');

    vi.advanceTimersByTime(1000);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
    expect(inset?.value).toBe('37');
    expect(topInset?.value).toBe('78');
  });

  it('keeps an already revealed header segment stable while toolbar expansion fills the remaining chrome', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();
    const identity = document.querySelector<HTMLElement>('[data-chrome-identity]');
    expect(identity?.style.opacity).toBe('1');
    expect(identity?.style.maskImage).toBe('none');

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(116);
    await tick();

    const headerSurface = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(identity?.style.opacity).toBe('1');
    expect(identity?.style.maskImage).toBe('none');
    expect(headerSurface?.style.maskImage).toContain('radial-gradient');
    expect(headerSurface?.style.maskImage).toContain('linear-gradient');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.maskImage).toContain('radial-gradient');

    const toolbarEffects = document.querySelector<HTMLElement>('[data-zen-pane-chrome-toolbar-effects]');
    expect(toolbarEffects?.style.transition).toContain('520ms');
    vi.advanceTimersByTime(560);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
  });

  it('keeps an already unified header surface connected while the toolbar expands', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(1900);
    await tick();

    const headerSurface = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-lane-surface]');
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
    expect(headerSurface?.style.maskImage).toBe('linear-gradient(black, black)');

    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(116);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
    expect(headerSurface?.style.maskImage).toBe('linear-gradient(black, black)');
  });

  it('keeps the two-stage toolbar reveal running when visible chrome or an attachment takes hold', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(1900);
    move(root, rect.left + 300, rect.top + 56);
    vi.advanceTimersByTime(116);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');

    (component as typeof component & { holdActions(): void }).holdActions();
    (component as typeof component & { holdChromeAttachment(): void }).holdChromeAttachment();
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('expanding');
  });

  it('keeps the pointer light active when the pointer returns to an already revealed header gap', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(1900);
    move(root, rect.left + 300, rect.top + 140);
    move(root, rect.left + 340, rect.top + 18);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
    expect(document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-engaged-spot]')?.style.opacity).toBe('1');
  });

  it('cancels the shared fade when the pointer returns to revealed chrome', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(1900);
    move(root, rect.left + 300, rect.top + 140);
    vi.advanceTimersByTime(1500);
    await tick();
    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('fading');

    move(root, rect.left + 300, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('held');
    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('1');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('1');
  });

  it('releases an open-menu hold when its segment is removed', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');

    (component as typeof component & { openActionsMenu(): void }).openActionsMenu();
    await tick();
    await vi.advanceTimersByTimeAsync(0);
    (component as typeof component & { removeActions(): void }).removeActions();
    await tick();
    root.dispatchEvent(new PointerEvent('pointerleave', { bubbles: true, pointerType: 'mouse' }));
    vi.advanceTimersByTime(3400);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('idle');
  });

  it('keeps the trailing segment surface anchored after the pane width changes', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.right - 40, rect.top + 18);
    vi.advanceTimersByTime(500);
    await tick();

    const surface = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-surface="actions"]');
    const header = document.querySelector<HTMLElement>('[data-zen-pane-chrome-header-effects]');
    expect(surface).not.toBeNull();
    expect(header).not.toBeNull();

    root.style.width = '480px';
    await vi.advanceTimersByTimeAsync(16);
    await tick();

    expect(surface?.getBoundingClientRect().right).toBe(header?.getBoundingClientRect().right);
  });

  it('recomputes the middle gap hit target after the pane widens', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');

    root.style.width = '720px';
    await vi.advanceTimersByTimeAsync(16);
    await tick();
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 480, rect.top + 18);
    vi.advanceTimersByTime(400);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('spot');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
  });

  it('ignores touch pointer intent', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    if (!root) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(root, rect.left + 40, rect.top + 18, 'touch');
    vi.advanceTimersByTime(1000);
    await tick();

    expect(document.querySelector<HTMLElement>('[data-chrome-identity]')?.style.opacity).toBe('0');
  });

  it('ignores pointer intent owned by an overlapping panel', async () => {
    component = mount(ZenModePaneChromeTestRoot, { target: document.body });
    await tick();
    const root = document.querySelector<HTMLElement>('[data-chrome-root]');
    const exclusion = document.querySelector<HTMLElement>('[data-chrome-reveal-exclusion]');
    if (!root || !exclusion) throw new Error('Missing pane chrome fixture');
    const rect = root.getBoundingClientRect();

    move(exclusion, rect.right - 40, rect.top + 18);
    vi.advanceTimersByTime(1000);
    await tick();

    expect(document.querySelector<HTMLOutputElement>('[data-chrome-phase]')?.value).toBe('idle');
    expect(document.querySelector<HTMLElement>('[data-chrome-actions]')?.style.opacity).toBe('0');
  });
});
