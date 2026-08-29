import '../../../../app.css';
import '@typie/lib/dayjs';

import { getAppContext } from '@typie/ui/context';
import { mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetChangelogCache } from './changelog-state.svelte';
import ChangelogModal from './ChangelogModal.svelte';

vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn(), tryAppContext: vi.fn() }));

const TOTAL_PAGES = 4;

const app = {
  state: { changelogOpen: true },
};

const entryFor = (page: number) => ({
  id: `c${page}`,
  title: `제목 ${page}`,
  date: '2026-08-20T00:00:00.000Z',
  image: null,
  body: '본문',
});

const pagedFetch = () =>
  vi.fn((url: string) => {
    const page = Number(new URL(url, 'https://example.com').searchParams.get('page'));

    return Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ entries: [entryFor(page)], hasMore: page < TOTAL_PAGES }),
    });
  });

const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

let component: Record<string, unknown> | undefined;

beforeEach(() => {
  resetChangelogCache();
  app.state.changelogOpen = true;
  vi.mocked(getAppContext).mockReturnValue(app as never);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
  resetChangelogCache();
  vi.unstubAllGlobals();
});

describe('changelog list auto-fill', () => {
  it('recreates the sentinel for each page so a short list keeps loading without a scroll', async () => {
    const fetchSpy = pagedFetch();
    vi.stubGlobal('fetch', fetchSpy);

    const target = document.createElement('div');
    document.body.append(target);
    component = mount(ChangelogModal, { target, intro: false });

    await vi.waitFor(() => expect(document.querySelectorAll('h2')).toHaveLength(TOTAL_PAGES));

    expect(fetchSpy.mock.calls.map(([url]) => url)).toEqual([
      '/api/changelog?page=1',
      '/api/changelog?page=2',
      '/api/changelog?page=3',
      '/api/changelog?page=4',
    ]);
    expect([...document.querySelectorAll('h2')].map((heading) => heading.textContent)).toEqual(['제목 1', '제목 2', '제목 3', '제목 4']);
  });

  it('stops requesting once the server reports no more pages', async () => {
    const fetchSpy = pagedFetch();
    vi.stubGlobal('fetch', fetchSpy);

    const target = document.createElement('div');
    document.body.append(target);
    component = mount(ChangelogModal, { target, intro: false });

    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(TOTAL_PAGES));

    await nextFrame();
    await nextFrame();
    await nextFrame();

    expect(fetchSpy).toHaveBeenCalledTimes(TOTAL_PAGES);
  });
});
