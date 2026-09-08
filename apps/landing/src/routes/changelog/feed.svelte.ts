import { SvelteSet } from 'svelte/reactivity';
import { fetchPage } from './changelog';
import type { ChangelogEntry, ChangelogPage } from './changelog';

export class Feed {
  entries = $state<ChangelogEntry[]>([]);
  hasMore = $state(true);
  loading = $state(false);
  failed = $state(false);
  nextPage = $state(2);

  loadMore = async () => {
    if (this.loading || !this.hasMore) return;
    this.loading = true;
    try {
      const page = await fetchPage(this.nextPage);
      if (page === null) return;
      const known = new SvelteSet(this.entries.map((entry) => entry.id));
      this.entries = [...this.entries, ...page.entries.filter((entry) => !known.has(entry.id))];
      this.hasMore = page.hasMore;
      this.nextPage += 1;
    } finally {
      this.loading = false;
    }
  };

  observe = (sentinel: HTMLElement) => {
    const observer = new IntersectionObserver(
      (records) => {
        if (records.some((record) => record.isIntersecting)) void this.loadMore();
      },
      { rootMargin: '0px 0px 600px 0px' },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  };

  constructor(initial: ChangelogPage | null) {
    if (initial) {
      this.entries = initial.entries;
      this.hasMore = initial.hasMore;
    } else {
      this.failed = true;
      this.hasMore = false;
    }
  }
}
