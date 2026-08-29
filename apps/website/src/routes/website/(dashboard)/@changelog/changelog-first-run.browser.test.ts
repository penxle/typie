import '../../../../app.css';
import '@typie/lib/dayjs';

import { getAppContext } from '@typie/ui/context';
import { mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetChangelogCache } from './changelog-state.svelte';
import ChangelogPopover from './ChangelogPopover.svelte';

vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn(), tryAppContext: vi.fn() }));
vi.mock('mixpanel-browser', () => ({ default: { track: vi.fn() } }));

const entry = {
  id: 'c9',
  title: '제목 9',
  date: '2026-08-20T00:00:00.000Z',
  image: null,
};

const app = {
  preference: { current: { changelogSeenId: '' } },
  state: { changelogOpen: false },
};

const highlightFetch = () => vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve({ entry }) }));

const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

let component: Record<string, unknown> | undefined;

beforeEach(() => {
  resetChangelogCache();
  app.preference.current.changelogSeenId = '';
  app.state.changelogOpen = false;
  vi.mocked(getAppContext).mockReturnValue(app as never);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
  resetChangelogCache();
  vi.unstubAllGlobals();
});

describe('changelog popover first run', () => {
  it('seeds the seen id and stays hidden for a reader who has none, so rollout day does not greet every existing user with an already shipped note', async () => {
    vi.stubGlobal('fetch', highlightFetch());

    const target = document.createElement('div');
    document.body.append(target);
    component = mount(ChangelogPopover, { target, props: { suppressed: false }, intro: false });

    await vi.waitFor(() => expect(app.preference.current.changelogSeenId).toBe('c9'));

    await nextFrame();
    await nextFrame();

    expect(target.querySelectorAll('button')).toHaveLength(0);
  });

  it('leaves an existing seen id alone and shows the popover, so seeding only ever happens on the very first run', async () => {
    app.preference.current.changelogSeenId = 'c8';

    vi.stubGlobal('fetch', highlightFetch());

    const target = document.createElement('div');
    document.body.append(target);
    component = mount(ChangelogPopover, { target, props: { suppressed: false }, intro: false });

    await vi.waitFor(() => expect(target.querySelectorAll('button').length).toBeGreaterThan(0));

    expect(app.preference.current.changelogSeenId).toBe('c8');
  });
});
