import { SvelteSet } from 'svelte/reactivity';
import { backoffDelay } from './backoff.ts';

export const BACKOFF_MS = [1000, 3000, 10_000, 30_000];

export type AutoResolveDeps = {
  resolve: (toolCallId: string) => Promise<void>;
  settled?: (error: unknown) => boolean;
  delays?: readonly number[];
  schedule?: (fn: () => void, ms: number) => ReturnType<typeof setTimeout>;
  clear?: (timer: ReturnType<typeof setTimeout>) => void;
};

type Entry = {
  attempts: number;
  timer: ReturnType<typeof setTimeout> | null;
};

export class AutoResolver {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  readonly #entries = new Map<string, Entry>();
  readonly #resolve: (toolCallId: string) => Promise<void>;
  readonly #settled: (error: unknown) => boolean;
  readonly #delays: readonly number[];
  readonly #schedule: (fn: () => void, ms: number) => ReturnType<typeof setTimeout>;
  readonly #clear: (timer: ReturnType<typeof setTimeout>) => void;

  readonly failedIds = new SvelteSet<string>();

  constructor(deps: AutoResolveDeps) {
    this.#resolve = deps.resolve;
    this.#settled = deps.settled ?? (() => false);
    this.#delays = deps.delays ?? BACKOFF_MS;
    this.#schedule = deps.schedule ?? ((fn, ms) => setTimeout(fn, ms));
    this.#clear = deps.clear ?? ((timer) => clearTimeout(timer));
  }

  #finish(toolCallId: string, entry: Entry) {
    entry.timer = null;
    this.failedIds.delete(toolCallId);
  }

  #attempt(toolCallId: string, entry: Entry) {
    entry.timer = null;
    entry.attempts += 1;

    void this.#resolve(toolCallId)
      .then(() => {
        if (this.#entries.get(toolCallId) === entry) {
          this.#finish(toolCallId, entry);
        }
      })
      .catch((err: unknown) => {
        if (this.#entries.get(toolCallId) !== entry) {
          return;
        }

        if (this.#settled(err)) {
          this.#finish(toolCallId, entry);
          return;
        }

        const delay = backoffDelay(this.#delays, entry.attempts);
        if (delay === null) {
          this.failedIds.add(toolCallId);
          return;
        }

        entry.timer = this.#schedule(() => {
          if (this.#entries.get(toolCallId) === entry) {
            this.#attempt(toolCallId, entry);
          }
        }, delay);
      });
  }

  request(toolCallId: string) {
    if (this.#entries.has(toolCallId)) {
      return;
    }

    const entry: Entry = { attempts: 0, timer: null };
    this.#entries.set(toolCallId, entry);
    this.#attempt(toolCallId, entry);
  }

  retry(toolCallId: string) {
    this.forget(toolCallId);
    this.request(toolCallId);
  }

  forget(toolCallId: string) {
    const entry = this.#entries.get(toolCallId);
    if (entry?.timer != null) {
      this.#clear(entry.timer);
    }

    this.#entries.delete(toolCallId);
    this.failedIds.delete(toolCallId);
  }

  retain(toolCallIds: readonly string[]) {
    for (const toolCallId of this.#entries.keys()) {
      if (!toolCallIds.includes(toolCallId)) {
        this.forget(toolCallId);
      }
    }
  }

  reset() {
    for (const toolCallId of this.#entries.keys()) {
      this.forget(toolCallId);
    }
  }
}
