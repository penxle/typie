import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchHighlight, fetchPage, resetChangelogCache, seenIdFor, shouldShowPopover } from './changelog-state.svelte';
import type { ChangelogHighlight } from './changelog-state.svelte';

const highlight = (id: string): ChangelogHighlight => ({
  id,
  title: '새 기능이 추가되었어요',
  date: '2026-08-20T00:00:00.000Z',
  image: null,
});

describe('shouldShowPopover', () => {
  it('shows when the highlighted entry differs from the seen id', () => {
    expect(shouldShowPopover(highlight('c2'), 'c1')).toBe(true);
  });

  it('does not show when the highlighted entry matches the seen id', () => {
    expect(shouldShowPopover(highlight('c1'), 'c1')).toBe(false);
  });

  it('does not show when there is no highlighted entry', () => {
    expect(shouldShowPopover(null, 'c1')).toBe(false);
    expect(shouldShowPopover(null, '')).toBe(false);
  });

  it('does not show for a first-time user, who has no seen id yet', () => {
    expect(shouldShowPopover(highlight('c1'), '')).toBe(false);
  });

  it('shows a back-dated entry whose highlight was turned on later', () => {
    const backDated: ChangelogHighlight = { ...highlight('c9'), date: '2020-01-01T00:00:00.000Z' };
    expect(shouldShowPopover(backDated, 'c1')).toBe(true);
  });
});

describe('seenIdFor', () => {
  it('returns the entry id', () => {
    expect(seenIdFor(highlight('c3'))).toBe('c3');
  });

  it('returns a sentinel when there is no highlighted entry', () => {
    expect(seenIdFor(null)).toBe('-');
  });
});

describe('fetchHighlight', () => {
  beforeEach(() => {
    resetChangelogCache();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns the entry from the API', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ entry: highlight('c1') }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchHighlight()).resolves.toMatchObject({ id: 'c1' });
    expect(fetchSpy).toHaveBeenCalledWith('/api/changelog?highlight=1');
  });

  it('caches the result so a second call does not hit the network', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ entry: highlight('c1') }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchHighlight()).resolves.toMatchObject({ id: 'c1' });
    await expect(fetchHighlight()).resolves.toMatchObject({ id: 'c1' });

    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it('returns null when the request fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({ entry: null }) }));

    await expect(fetchHighlight()).resolves.toBeNull();
  });

  it('returns null when fetch throws', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));

    await expect(fetchHighlight()).resolves.toBeNull();
  });

  it('retries after a failure instead of caching the empty result', async () => {
    const fetchSpy = vi
      .fn()
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue({ ok: true, json: async () => ({ entry: highlight('c1') }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchHighlight()).resolves.toBeNull();
    await expect(fetchHighlight()).resolves.toMatchObject({ id: 'c1' });
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('retries after a failed response instead of caching the empty result', async () => {
    const fetchSpy = vi
      .fn()
      .mockResolvedValueOnce({ ok: false, json: async () => ({ entry: null }) })
      .mockResolvedValue({ ok: true, json: async () => ({ entry: highlight('c1') }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchHighlight()).resolves.toBeNull();
    await expect(fetchHighlight()).resolves.toMatchObject({ id: 'c1' });
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });
});

describe('fetchPage', () => {
  beforeEach(() => {
    resetChangelogCache();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('requests the given page and returns its entries', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        entries: [{ id: 'c1', title: '제목', date: '2026-08-20T00:00:00.000Z', image: null, body: '본문' }],
        hasMore: true,
      }),
    });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchPage(2)).resolves.toEqual({
      entries: [{ id: 'c1', title: '제목', date: '2026-08-20T00:00:00.000Z', image: null, body: '본문' }],
      hasMore: true,
    });
    expect(fetchSpy).toHaveBeenCalledWith('/api/changelog?page=2');
  });

  it('caches each page separately', async () => {
    const entry = { id: 'c1', title: '제목', date: '2026-08-20T00:00:00.000Z', image: null, body: '본문' };
    const fetchSpy = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ entries: [entry], hasMore: false }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchPage(1)).resolves.toEqual({ entries: [entry], hasMore: false });
    await expect(fetchPage(1)).resolves.toEqual({ entries: [entry], hasMore: false });
    await expect(fetchPage(2)).resolves.toEqual({ entries: [entry], hasMore: false });

    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('returns null when fetch throws', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));

    await expect(fetchPage(1)).resolves.toBeNull();
  });

  it('retries after a throw instead of caching the failure', async () => {
    const entry = { id: 'c1', title: '제목', date: '2026-08-20T00:00:00.000Z', image: null, body: '본문' };
    const fetchSpy = vi
      .fn()
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue({ ok: true, json: async () => ({ entries: [entry], hasMore: false }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchPage(1)).resolves.toBeNull();
    await expect(fetchPage(1)).resolves.toEqual({ entries: [entry], hasMore: false });
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('retries after a failed response instead of caching the failure', async () => {
    const entry = { id: 'c1', title: '제목', date: '2026-08-20T00:00:00.000Z', image: null, body: '본문' };
    const fetchSpy = vi
      .fn()
      .mockResolvedValueOnce({ ok: false, json: async () => ({ entries: [], hasMore: false }) })
      .mockResolvedValue({ ok: true, json: async () => ({ entries: [entry], hasMore: false }) });
    vi.stubGlobal('fetch', fetchSpy);

    await expect(fetchPage(1)).resolves.toBeNull();
    await expect(fetchPage(1)).resolves.toEqual({ entries: [entry], hasMore: false });
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });
});
