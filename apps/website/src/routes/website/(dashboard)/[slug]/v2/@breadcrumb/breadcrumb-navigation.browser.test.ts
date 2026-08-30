import '../../../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { userEvent } from 'vitest/browser';
import BreadcrumbDocumentDragTestRoot from './breadcrumb-document-drag-test-root.svelte';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const popup = () => {
  const expandedTrigger = document.querySelector<HTMLElement>('[aria-haspopup="tree"][aria-expanded="true"]');
  const treeId = expandedTrigger?.getAttribute('aria-controls');
  const tree = treeId ? document.querySelector<HTMLElement>(`[id="${CSS.escape(treeId)}"]`) : null;
  return tree?.getAttribute('role') === 'tree' ? tree.parentElement?.parentElement : null;
};
const treeItem = (entityId: string) => document.querySelector<HTMLElement>(`[role="treeitem"][data-breadcrumb-entity-id="${entityId}"]`);
const trigger = (name: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('[aria-haspopup="tree"]')].find((button) => button.textContent?.includes(name));

const mountFixture = async () => {
  component = mount(BreadcrumbDocumentDragTestRoot, { target: document.body });
  await tick();
};

describe('breadcrumb navigation keyboard interaction', () => {
  it('marks the current document after expanding it from an ancestor segment', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing folder breadcrumb trigger');

    folderTrigger.focus();
    await userEvent.keyboard('{Enter}');
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));

    await userEvent.keyboard('{ArrowRight}');
    await expect.poll(() => treeItem('document-current')).not.toBeNull();
    await userEvent.keyboard('{ArrowRight}');
    await expect.poll(() => document.activeElement).toBe(treeItem('document-sibling'));
    await userEvent.keyboard('{ArrowDown}');
    await expect.poll(() => document.activeElement).toBe(treeItem('document-current'));

    expect(treeItem('folder-1')?.getAttribute('aria-current')).toBeNull();
    expect(treeItem('folder-1')?.getAttribute('aria-selected')).toBe('true');
    expect(treeItem('document-current')?.getAttribute('aria-current')).toBe('page');
    expect(treeItem('document-current')?.getAttribute('aria-selected')).toBe('false');
  });

  it('opens with Space and supports tree navigation and document activation from actual focus', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing folder breadcrumb trigger');

    folderTrigger.focus();
    await userEvent.keyboard(' ');
    await expect.poll(popup).not.toBeNull();
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));

    await userEvent.keyboard('{End}');
    await expect.poll(() => document.activeElement).toBe(treeItem('document-extra-15'));
    await userEvent.keyboard('{Home}');
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));

    await userEvent.keyboard('{ArrowRight}');
    await expect.poll(() => treeItem('document-current')).not.toBeNull();
    await userEvent.keyboard('{ArrowRight}');
    await expect.poll(() => document.activeElement).toBe(treeItem('document-sibling'));
    await userEvent.keyboard('{ArrowLeft}');
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));
    await userEvent.keyboard('{ArrowLeft}');
    await expect.poll(() => treeItem('folder-1')?.getAttribute('aria-expanded')).toBe('false');

    await userEvent.keyboard('{ArrowDown}');
    await expect.poll(() => document.activeElement).toBe(treeItem('document-first'));
    await userEvent.keyboard('{Enter}');
    await expect.poll(popup).toBeNull();
    await expect.poll(() => document.querySelector('[data-navigated-slug]')?.textContent).toBe('document-first');
  });

  it('moves from automatic tree focus with row highlighting instead of a focus ring', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing folder breadcrumb trigger');

    await userEvent.click(folderTrigger);
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));

    await userEvent.keyboard('{ArrowDown}');
    const nextItem = treeItem('document-first');
    await expect.poll(() => document.activeElement).toBe(nextItem);
    expect(nextItem?.matches(':focus-visible')).toBe(false);

    const row = nextItem?.querySelector<HTMLElement>('[data-breadcrumb-tree-row]');
    const ordinaryRow = treeItem('document-extra-0')?.querySelector<HTMLElement>('[data-breadcrumb-tree-row]');
    if (!nextItem || !row || !ordinaryRow) throw new Error('Missing breadcrumb rows');
    expect(getComputedStyle(row).backgroundColor).not.toBe(getComputedStyle(ordinaryRow).backgroundColor);
  });

  it('closes on Tab without restoring focus to the segment trigger', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing breadcrumb trigger');

    await userEvent.click(folderTrigger);
    await expect.poll(() => document.activeElement).toBe(treeItem('folder-1'));

    await userEvent.keyboard('{Tab}');
    await expect.poll(popup).toBeNull();
    expect(document.activeElement).not.toBe(folderTrigger);
    expect(document.querySelector('[data-holds]')?.textContent).toBe('[]');
  });

  it('keeps the open segment trigger styled like hover', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    const currentTrigger = trigger('Current document');
    if (!folderTrigger || !currentTrigger) throw new Error('Missing breadcrumb triggers');

    await userEvent.click(folderTrigger);
    const mountedPopup = popup();
    if (!mountedPopup) throw new Error('Missing breadcrumb popup');

    const closedBackground = getComputedStyle(currentTrigger).backgroundColor;
    const closedColor = getComputedStyle(currentTrigger).color;
    await userEvent.click(currentTrigger);
    await userEvent.unhover(currentTrigger);
    expect(currentTrigger.getAttribute('aria-expanded')).toBe('true');
    expect(popup()).toBe(mountedPopup);
    await expect.poll(() => mountedPopup.getBoundingClientRect().left).toBeCloseTo(currentTrigger.getBoundingClientRect().left, 0);
    expect(getComputedStyle(currentTrigger).backgroundColor).not.toBe(closedBackground);
    expect(getComputedStyle(currentTrigger).color).not.toBe(closedColor);
  });
});
