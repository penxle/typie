import '../../../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { DocumentPaneDragController } from '../../@pane/document-pane-drag.svelte';
import BreadcrumbDocumentDragTestRoot from './breadcrumb-document-drag-test-root.svelte';
import type { PaneGroup } from '../../@pane/context.svelte';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const output = (name: string) => document.querySelector(`[data-${name}]`)?.textContent ?? '';
const popup = () => {
  const expandedTrigger = document.querySelector<HTMLElement>('[aria-haspopup="tree"][aria-expanded="true"]');
  const treeId = expandedTrigger?.getAttribute('aria-controls');
  const tree = treeId ? document.querySelector<HTMLElement>(`[id="${CSS.escape(treeId)}"]`) : null;
  return tree?.getAttribute('role') === 'tree' ? tree.parentElement?.parentElement : null;
};
const ghost = () =>
  [...document.body.children].find(
    (element): element is HTMLElement =>
      element instanceof HTMLElement &&
      element.dataset.portal !== undefined &&
      element.getAttribute('aria-hidden') === 'true' &&
      element.getAttribute('role') === 'presentation' &&
      element.textContent?.includes('First document') === true,
  ) ?? null;
const treeItem = (entityId: string) => document.querySelector<HTMLElement>(`[role="treeitem"][data-breadcrumb-entity-id="${entityId}"]`);

const installPointerCapture = (element: HTMLElement) => {
  let capturedPointerId: number | undefined;
  element.setPointerCapture = (pointerId) => (capturedPointerId = pointerId);
  element.hasPointerCapture = (pointerId) => capturedPointerId === pointerId;
  element.releasePointerCapture = (pointerId) => {
    if (capturedPointerId === pointerId) capturedPointerId = undefined;
  };
};

const pointer = (
  target: EventTarget,
  type: 'pointerdown' | 'pointermove' | 'pointerup' | 'pointercancel' | 'lostpointercapture',
  { pointerType = 'mouse', x = 20, y = 20, pointerId = 1 }: { pointerType?: string; x?: number; y?: number; pointerId?: number } = {},
) => {
  const event = new PointerEvent(type, {
    bubbles: true,
    cancelable: true,
    isPrimary: true,
    button: type === 'pointermove' ? -1 : 0,
    buttons: type === 'pointerup' || type === 'pointercancel' ? 0 : 1,
    clientX: x,
    clientY: y,
    pointerId,
    pointerType,
  });
  target.dispatchEvent(event);
  return event;
};

const openPopup = async () => {
  document.querySelector<HTMLButtonElement>('[aria-haspopup="tree"]')?.click();
  await expect.poll(popup).not.toBeNull();
  await expect.poll(() => treeItem('document-first')).not.toBeNull();
  await new Promise((resolve) => setTimeout(resolve, 20));
};

const mountFixture = async () => {
  component = mount(BreadcrumbDocumentDragTestRoot, { target: document.body });
  await tick();
  await openPopup();
};

const beginActiveDrag = async () => {
  const item = treeItem('document-first');
  if (!item) throw new Error('Missing document treeitem');
  installPointerCapture(item);
  pointer(item, 'pointerdown', { x: 20, y: 20 });
  pointer(item, 'pointermove', { x: 31, y: 20 });
  await tick();
  return item;
};

describe('breadcrumb document drag', () => {
  it('leaves nested interactive controls outside the drag gesture', () => {
    const scrollSurface = document.createElement('div');
    scrollSurface.dataset.documentPaneDragScrollSurface = '';
    const row = document.createElement('a');
    const button = document.createElement('button');
    row.append(button);
    scrollSurface.append(row);
    document.body.append(scrollSurface);
    installPointerCapture(row);

    const controller = new DocumentPaneDragController({ paneGroup: {} as PaneGroup });
    const action = controller.drag(row, { slug: 'document-first', name: 'First document' });

    pointer(button, 'pointerdown');

    expect(controller.hasPointerSession).toBe(false);
    action?.destroy?.();
    controller.destroy();
  });

  it('keeps a below-threshold mouse gesture as document activation', async () => {
    await mountFixture();
    const item = treeItem('document-first');
    if (!item) throw new Error('Missing document treeitem');

    pointer(item, 'pointerdown', { x: 20, y: 20 });
    pointer(item, 'pointermove', { x: 25, y: 25 });
    pointer(item, 'pointerup', { x: 25, y: 25 });
    item.click();

    await expect.poll(() => output('navigated-slug')).toBe('document-first');
    expect(output('pane-executions')).toBe('[]');
  });

  it('delegates a successful drop to PaneGroup without activating the source pane', async () => {
    await mountFixture();
    const item = await beginActiveDrag();
    expect(ghost()).not.toBeNull();

    pointer(item, 'pointerup', { x: 31, y: 20 });
    item.click();

    await expect.poll(popup).toBeNull();
    expect(output('pane-executions')).toContain('"zone":"center"');
    expect(output('pane-executions')).toContain('"slug":"document-first"');
    expect(output('holds')).toBe('[]');
    expect(output('navigated-slug')).toBe('');
  });

  it('starts touch drag after the shared hold and suppresses contextmenu while pending', async () => {
    await mountFixture();
    const item = treeItem('document-first');
    if (!item) throw new Error('Missing document treeitem');
    installPointerCapture(item);

    pointer(item, 'pointerdown', { pointerType: 'touch' });
    const pendingMenu = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    item.dispatchEvent(pendingMenu);
    expect(pendingMenu.defaultPrevented).toBe(true);
    expect(ghost()).toBeNull();

    await new Promise((resolve) => setTimeout(resolve, 370));
    await expect.poll(ghost).not.toBeNull();
    pointer(item, 'pointermove', { pointerType: 'touch', x: 45, y: 55 });
    const activeGhost = ghost();
    if (!activeGhost) throw new Error('Missing active document ghost');
    expect(getComputedStyle(activeGhost).userSelect).toBe('none');
    await expect.poll(() => output('pane-updates')).toContain('"x":45,"y":55');

    pointer(item, 'pointerup', { pointerType: 'touch' });
    document.body.click();
    await expect.poll(popup).toBeNull();
  });

  it('cancels touch hold on movement without suppressing later contextmenu', async () => {
    await mountFixture();
    const item = treeItem('document-first');
    if (!item) throw new Error('Missing document treeitem');

    const tree = item.closest<HTMLElement>('[role="tree"]');
    if (!tree) throw new Error('Missing tree scroll surface');
    expect(tree.scrollHeight).toBeGreaterThan(tree.clientHeight);
    tree.scrollTop = 100;

    pointer(item, 'pointerdown', { pointerType: 'touch', x: 20, y: 20 });
    pointer(item, 'pointermove', { pointerType: 'touch', x: 20, y: 5 });
    expect(tree.scrollTop).toBe(115);
    pointer(item, 'pointermove', { pointerType: 'touch', x: 20, y: -5 });
    expect(tree.scrollTop).toBe(125);
    await new Promise((resolve) => setTimeout(resolve, 370));

    expect(ghost()).toBeNull();
    expect(output('pane-updates')).toBe('[]');
    expect(output('pane-cancel-count')).toBe('0');
    const menu = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    item.dispatchEvent(menu);
    expect(menu.defaultPrevented).toBe(false);

    pointer(item, 'pointerup', { pointerType: 'touch', x: 20, y: -5 });
    item.click();
    await tick();
    expect(output('navigated-slug')).toBe('');
    expect(popup()).not.toBeNull();

    treeItem('document-extra-0')?.click();
    await expect.poll(() => output('navigated-slug')).toBe('document-extra-0');
  });

  it('keeps the popup open when there is no pane drop zone', async () => {
    await mountFixture();
    const select = document.querySelector<HTMLSelectElement>('[data-next-drop-zone]');
    if (!select) throw new Error('Missing zone control');
    select.value = 'none';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await tick();
    const item = await beginActiveDrag();

    pointer(item, 'pointerup', { x: 31, y: 20 });
    item.click();

    await expect.poll(ghost).toBeNull();
    expect(popup()).not.toBeNull();
    expect(output('holds')).toContain('breadcrumb-popup');
    expect(output('pane-cancel-count')).toBe('1');
    expect(output('navigated-slug')).toBe('');
  });

  it('suppresses a delayed click after unexpected lost pointer capture', async () => {
    await mountFixture();
    const item = await beginActiveDrag();

    pointer(item, 'lostpointercapture', { x: 31, y: 20 });
    await expect.poll(ghost).toBeNull();
    await new Promise((resolve) => setTimeout(resolve, 20));
    pointer(item, 'pointerup', { x: 31, y: 20 });
    item.click();
    await tick();

    expect(output('navigated-slug')).toBe('');
    expect(popup()).not.toBeNull();
    treeItem('document-extra-0')?.click();
    await expect.poll(() => output('navigated-slug')).toBe('document-extra-0');
  });

  it('uses the first Escape to cancel drag and a later Escape to close and restore the trigger', async () => {
    await mountFixture();
    const item = await beginActiveDrag();
    const trigger = document.querySelector<HTMLButtonElement>('[aria-haspopup="tree"]');
    item.focus();
    expect(document.activeElement).toBe(item);

    item.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'Escape' }));
    await expect.poll(ghost).toBeNull();
    expect(popup()).not.toBeNull();
    expect(output('holds')).toContain('breadcrumb-popup');
    expect(output('pane-cancel-count')).toBe('1');

    item.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'Escape' }));
    await expect.poll(popup).toBeNull();
    await expect.poll(() => document.activeElement).toBe(trigger);
    expect(output('holds')).toBe('[]');
    expect(output('pane-cancel-count')).toBe('1');
  });

  it('suspends outside-click dismissal while pointer drag is pending', async () => {
    await mountFixture();
    const item = treeItem('document-first');
    if (!item) throw new Error('Missing document treeitem');
    installPointerCapture(item);

    pointer(item, 'pointerdown');
    document.body.click();
    expect(popup()).not.toBeNull();

    pointer(item, 'pointercancel', { x: 31, y: 20 });
  });
});
