import '../../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import PaneHeaderTestRoot from './pane-header-test-root.svelte';

vi.mock('mixpanel-browser', () => ({ default: { track: vi.fn() } }));

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  localStorage.clear();
  document.body.replaceChildren();
});

describe('PaneHeader priority lanes', () => {
  it('signals a transient sidebar peek while the top-left control is hovered', async () => {
    component = mount(PaneHeaderTestRoot, { target: document.body });
    await tick();

    const sidebar = document.querySelector<HTMLButtonElement>('[data-pane-header-test-host] [role="region"] button');
    const peek = document.querySelector<HTMLOutputElement>('[data-sidebar-peek]');
    const hidden = document.querySelector<HTMLOutputElement>('[data-sidebar-hidden]');
    if (!sidebar || !peek || !hidden) throw new Error('Missing sidebar peek test fixture');

    await userEvent.hover(sidebar);
    expect(peek.value).toBe('true');

    await userEvent.click(sidebar);
    expect(hidden.value).toBe('true');
    expect(peek.value).toBe('false');

    await userEvent.unhover(sidebar);
    expect(peek.value).toBe('false');
  });

  it('keeps global and fixed controls visible while panel actions scroll', async () => {
    component = mount(PaneHeaderTestRoot, { target: document.body });
    await tick();

    const host = document.querySelector<HTMLElement>('[data-pane-header-test-host]');
    const region = host?.querySelector<HTMLElement>('[role="region"]');
    const sidebar = region?.querySelector<HTMLButtonElement>('button');
    const prism = region?.querySelector<HTMLButtonElement>('[aria-label^="PRISM"]');
    const fixedActions = [...(region?.querySelectorAll<HTMLElement>('[data-pane-header-fixed-action]') ?? [])];
    const breadcrumbViewport = document.querySelector<HTMLElement>('#pane-header-breadcrumb-test');
    const scrollViewport = document.querySelector<HTMLElement>('#pane-header-actions-pane-header-test');

    expect(region).not.toBeNull();
    expect(sidebar).not.toBeNull();
    expect(prism).not.toBeNull();
    expect(fixedActions).toHaveLength(3);
    expect(breadcrumbViewport).not.toBeNull();
    expect(scrollViewport).not.toBeNull();

    if (!region || !sidebar || !prism || !breadcrumbViewport || !scrollViewport) throw new Error('Missing PaneHeader test fixture');

    const bounds = region.getBoundingClientRect();
    for (const control of [sidebar, ...fixedActions, prism]) {
      const rect = control.getBoundingClientRect();
      expect(rect.left).toBeGreaterThanOrEqual(bounds.left);
      expect(rect.right).toBeLessThanOrEqual(bounds.right);
    }
    expect(breadcrumbViewport.getBoundingClientRect().left - sidebar.getBoundingClientRect().right).toBeCloseTo(4, 1);

    await expect.poll(() => scrollViewport.scrollWidth).toBeGreaterThan(scrollViewport.clientWidth);
    const scrollbar = document.querySelector<HTMLElement>('[role="scrollbar"][aria-label="문서 헤더 도구 가로 스크롤"]');
    expect(scrollbar?.getAttribute('aria-controls')).toBe(scrollViewport.id);
    expect(scrollbar?.getBoundingClientRect().bottom).toBeCloseTo(bounds.bottom, 1);
  });

  it('places the breadcrumb fog at the pane edge and its scrollbar at the header bottom', async () => {
    component = mount(PaneHeaderTestRoot, { target: document.body });
    await tick();

    const host = document.querySelector<HTMLElement>('[data-pane-header-breadcrumb-test-host]');
    const region = host?.querySelector<HTMLElement>('[role="region"]');
    const viewport = document.querySelector<HTMLElement>('#pane-header-breadcrumb-only-test');
    const scrollbar = document.querySelector<HTMLElement>('[role="scrollbar"][aria-label="문서 경로 가로 스크롤"]');
    if (!region || !viewport || !scrollbar) throw new Error('Missing breadcrumb PaneHeader test fixture');

    await expect.poll(() => viewport.scrollWidth).toBeGreaterThan(viewport.clientWidth);
    const bounds = region.getBoundingClientRect();
    expect(viewport.getBoundingClientRect().left).toBeCloseTo(bounds.left + 4, 1);
    expect(scrollbar.getBoundingClientRect().bottom).toBeCloseTo(bounds.bottom, 1);
  });

  it('leaves header ownership to the loaded document while its body skeleton remains', async () => {
    component = mount(PaneHeaderTestRoot, { target: document.body });
    await tick();

    const host = document.querySelector<HTMLElement>('[data-pane-header-skeleton-handoff-test-host]');
    expect(host?.querySelectorAll('[role="region"]')).toHaveLength(1);
  });
});
