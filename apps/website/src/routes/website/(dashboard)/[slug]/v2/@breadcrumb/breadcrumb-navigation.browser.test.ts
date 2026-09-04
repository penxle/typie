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
const activeTree = () => popup()?.querySelector<HTMLElement>('[role="tree"]') ?? null;
const treeItem = (entityId: string) => document.querySelector<HTMLElement>(`[role="treeitem"][data-breadcrumb-entity-id="${entityId}"]`);
const treeItemByKey = (key: string) => document.querySelector<HTMLElement>(`[role="treeitem"][data-breadcrumb-tree-item-key="${key}"]`);
const trigger = (name: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('[aria-haspopup="tree"]')].find((button) => button.textContent?.includes(name));

type FixtureProps = {
  currentKind?: 'entity' | 'home';
  isOwner?: boolean;
  rootQueryLoading?: boolean;
};

const mountFixture = async (props: FixtureProps = {}) => {
  component = mount(BreadcrumbDocumentDragTestRoot, { target: document.body, props: { ...props, withSegment: false } });
  await tick();
};

describe('breadcrumb navigation keyboard interaction', () => {
  it('keeps the path segment structure and geometry when navigation is non-interactive', async () => {
    await mountFixture();
    const interactiveSegment = document.querySelector('nav[aria-label="문서 경로"] > ol > li:first-child > button');
    if (!(interactiveSegment instanceof HTMLElement)) throw new Error('Missing interactive breadcrumb segment');
    const interactiveStyle = getComputedStyle(interactiveSegment);
    const interactiveGeometry = {
      alignItems: interactiveStyle.alignItems,
      borderRadius: interactiveStyle.borderRadius,
      columnGap: interactiveStyle.columnGap,
      height: interactiveStyle.height,
      paddingLeft: interactiveStyle.paddingLeft,
      paddingRight: interactiveStyle.paddingRight,
    };

    if (!component) throw new Error('Missing mounted breadcrumb fixture');
    await unmount(component);
    component = undefined;
    document.body.replaceChildren();

    await mountFixture({ isOwner: false });
    const path = document.querySelector('nav[aria-label="문서 경로"] > ol');
    if (!path) throw new Error('Missing breadcrumb path');

    const segments = [...path.children];
    expect(segments).toHaveLength(2);
    expect(segments.every((segment) => segment.matches('li'))).toBe(true);
    expect(path.querySelector('button')).toBeNull();
    expect(segments[0]?.querySelector(':scope > span > span[aria-hidden="true"]')).not.toBeNull();
    expect(segments[1]?.querySelector(':scope > span > span[aria-hidden="true"]')).toBeNull();
    const passiveSegment = segments[0]?.querySelector(':scope > span');
    if (!(passiveSegment instanceof HTMLElement)) throw new Error('Missing non-interactive breadcrumb segment');
    const passiveStyle = getComputedStyle(passiveSegment);
    expect({
      alignItems: passiveStyle.alignItems,
      borderRadius: passiveStyle.borderRadius,
      columnGap: passiveStyle.columnGap,
      height: passiveStyle.height,
      paddingLeft: passiveStyle.paddingLeft,
      paddingRight: passiveStyle.paddingRight,
    }).toEqual(interactiveGeometry);
  });

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
    await expect.poll(() => document.activeElement).toBe(treeItemByKey('home'));
    await userEvent.keyboard('{ArrowDown}');
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

  it('closes when the open segment trigger is clicked again', async () => {
    await mountFixture();
    const currentTrigger = trigger('Current document');
    if (!currentTrigger) throw new Error('Missing current breadcrumb trigger');

    await userEvent.click(currentTrigger);
    await expect.poll(popup).not.toBeNull();

    await userEvent.click(currentTrigger);
    await expect.poll(popup).toBeNull();
    expect(currentTrigger.getAttribute('aria-expanded')).toBe('false');
  });

  it('puts Home first in the root entity popup and activates it as a navigation target', async () => {
    await mountFixture();
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing root breadcrumb trigger');

    await userEvent.click(folderTrigger);
    const tree = activeTree();
    const homeItem = treeItemByKey('home');
    if (!tree || !homeItem) throw new Error('Missing root breadcrumb tree or Home item');

    expect(tree.querySelector('[role="treeitem"]')).toBe(homeItem);
    await userEvent.click(homeItem);
    await expect.poll(popup).toBeNull();
    await expect.poll(() => document.querySelector('[data-navigation-target]')?.textContent).toBe('{"kind":"home"}');
  });

  it('does not repeat Home inside the Home segment popup', async () => {
    await mountFixture({ currentKind: 'home' });
    const homeTrigger = trigger('홈');
    if (!homeTrigger) throw new Error('Missing Home breadcrumb trigger');

    await userEvent.click(homeTrigger);
    await expect.poll(activeTree).not.toBeNull();
    expect(activeTree()?.querySelector('[data-breadcrumb-tree-item-key="home"]')).toBeNull();
  });

  it('does not add Home to a nested entity popup', async () => {
    await mountFixture();
    const currentTrigger = trigger('Current document');
    if (!currentTrigger) throw new Error('Missing nested breadcrumb trigger');

    await userEvent.click(currentTrigger);
    await expect.poll(activeTree).not.toBeNull();
    expect(activeTree()?.querySelector('[data-breadcrumb-tree-item-key="home"]')).toBeNull();
  });

  it('keeps Home available while the root entity query is loading', async () => {
    await mountFixture({ rootQueryLoading: true });
    const folderTrigger = trigger('Folder');
    if (!folderTrigger) throw new Error('Missing root breadcrumb trigger');

    await userEvent.click(folderTrigger);
    const homeItem = treeItemByKey('home');
    if (!homeItem) throw new Error('Missing Home item while the root query is loading');

    await userEvent.click(homeItem);
    await expect.poll(() => document.querySelector('[data-navigation-target]')?.textContent).toBe('{"kind":"home"}');
  });
});
