export type ChangelogEntry = {
  id: string;
  title: string;
  date: string;
  image: { url: string } | null;
  body: string;
};

export type ChangelogHighlight = {
  id: string;
  title: string;
  date: string;
  image: { url: string } | null;
};

export type ChangelogPage = {
  entries: ChangelogEntry[];
  hasMore: boolean;
};

const NO_HIGHLIGHT = '-';

let highlightCache: { entry: ChangelogHighlight | null } | null = null;
let highlightInflight: Promise<ChangelogHighlight | null> | null = null;

// eslint-disable-next-line svelte/prefer-svelte-reactivity
const pageCache = new Map<number, ChangelogPage>();
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const pageInflight = new Map<number, Promise<ChangelogPage | null>>();

export const resetChangelogCache = () => {
  highlightCache = null;
  highlightInflight = null;
  pageCache.clear();
  pageInflight.clear();
};

export const fetchHighlight = async (): Promise<ChangelogHighlight | null> => {
  if (highlightCache) {
    return highlightCache.entry;
  }

  if (highlightInflight) {
    return highlightInflight;
  }

  highlightInflight = (async () => {
    try {
      const response = await fetch('/api/changelog?highlight=1');
      if (!response.ok) return null;

      const data = (await response.json()) as { entry: ChangelogHighlight | null };
      highlightCache = { entry: data.entry };
      return data.entry;
    } catch {
      return null;
    } finally {
      highlightInflight = null;
    }
  })();

  return highlightInflight;
};

export const fetchPage = async (page: number): Promise<ChangelogPage | null> => {
  const cached = pageCache.get(page);
  if (cached) {
    return cached;
  }

  const inflight = pageInflight.get(page);
  if (inflight) {
    return inflight;
  }

  const loading = (async () => {
    try {
      const response = await fetch(`/api/changelog?page=${page}`);
      if (!response.ok) return null;

      const data = (await response.json()) as ChangelogPage;
      pageCache.set(page, data);
      return data;
    } catch {
      return null;
    } finally {
      pageInflight.delete(page);
    }
  })();

  pageInflight.set(page, loading);

  return loading;
};

export const seenIdFor = (highlight: ChangelogHighlight | null): string => highlight?.id ?? NO_HIGHLIGHT;

export const shouldShowPopover = (highlight: ChangelogHighlight | null, seenId: string): boolean => {
  if (!highlight) return false;
  if (seenId === '') return false;
  return highlight.id !== seenId;
};
