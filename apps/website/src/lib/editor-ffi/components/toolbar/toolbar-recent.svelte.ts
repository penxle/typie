import { safeJsonParse } from '@typie/ui/utils';
import { SvelteMap } from 'svelte/reactivity';

const LIMIT = 3;

const storageKey = (userId: string) => `typie:toolbar-recent:${userId}`;

class ToolbarRecentStore {
  #key: string;
  picks = $state<Record<string, string[]>>({});

  constructor(userId: string) {
    this.#key = storageKey(userId);
    this.picks = typeof localStorage === 'undefined' ? {} : safeJsonParse<Record<string, string[]>>(localStorage.getItem(this.#key), {});
  }

  ids(key: string): string[] {
    return this.picks[key] ?? [];
  }

  remember(key: string, id: string) {
    const current = this.picks[key] ?? [];
    if (current.includes(id)) return;

    this.picks[key] = [id, ...current].slice(0, LIMIT);

    try {
      localStorage.setItem(this.#key, JSON.stringify(this.picks));
    } catch {
      return;
    }
  }
}

const stores = new SvelteMap<string, ToolbarRecentStore>();

export const toolbarRecent = (userId: string): ToolbarRecentStore => {
  let store = stores.get(userId);
  if (!store) {
    store = new ToolbarRecentStore(userId);
    stores.set(userId, store);
  }
  return store;
};
