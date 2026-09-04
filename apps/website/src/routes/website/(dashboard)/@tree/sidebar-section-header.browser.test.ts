import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import SidebarSectionHeaderTestRoot from './sidebar-section-header-test-root.svelte';

let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const output = (name: string) => document.querySelector(`[data-${name}]`)?.textContent ?? '';
const tab = (label: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find((button) => button.textContent?.trim() === label);

const chevron = () => document.querySelector<HTMLButtonElement>('[aria-label$="열기/닫기"]');

const mountFixture = async () => {
  component = mount(SidebarSectionHeaderTestRoot, { target: document.body });
  await tick();
};

describe('sidebar section header tabs', () => {
  it('switches tabs without toggling when the section is open', async () => {
    await mountFixture();

    tab('고정')?.click();
    await tick();

    expect(output('active-tab')).toBe('PINNED');
    expect(output('open')).toBe('true');
    expect(output('toggles')).toBe('0');
    expect(tab('고정')?.getAttribute('aria-selected')).toBe('true');
    expect(tab('최근')?.getAttribute('aria-selected')).toBe('false');
  });

  it('toggles the section when the active tab is clicked', async () => {
    await mountFixture();

    tab('최근')?.click();
    await tick();

    expect(output('open')).toBe('false');
    expect(output('toggles')).toBe('1');
    expect(output('active-tab')).toBe('RECENT');
  });

  it('reopens a collapsed section when an inactive tab is selected', async () => {
    await mountFixture();

    tab('최근')?.click();
    await tick();
    expect(output('open')).toBe('false');

    tab('고정')?.click();
    await tick();

    expect(output('active-tab')).toBe('PINNED');
    expect(output('open')).toBe('true');
    expect(output('toggles')).toBe('2');
  });

  it('toggles the section from the chevron without changing the active tab', async () => {
    await mountFixture();

    chevron()?.click();
    await tick();

    expect(output('open')).toBe('false');
    expect(output('toggles')).toBe('1');
    expect(output('active-tab')).toBe('RECENT');

    chevron()?.click();
    await tick();

    expect(output('open')).toBe('true');
    expect(output('toggles')).toBe('2');
  });

  it('renders the actions snippet only while the parent shows it and marks the drop target tab', async () => {
    await mountFixture();

    expect(document.querySelector('[data-sort-menu]')).not.toBeNull();
    expect(tab('고정')?.dataset.dropTarget).toBe('pin');
    expect(tab('최근')?.dataset.dropTarget).toBeUndefined();

    tab('고정')?.click();
    await tick();

    expect(document.querySelector('[data-sort-menu]')).toBeNull();
  });
});
