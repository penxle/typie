import { wasm } from '$lib/wasm-ffi.svelte';
import { snapshot } from './registry';
import type { FontFamilySource } from '$mearie';
import type { EditorEventListener } from './types';

const CACHE_NAME = 'typie-fonts';
export const PRELOAD_CONCURRENCY = 4;
const REQUIRED_LOAD_ATTEMPTS = 3;
const PREFETCH_LOAD_ATTEMPTS = 1;
const LOAD_RETRY_BASE_MS = 200;
export const RETRY_MAX_ATTEMPTS = 5;
export const RETRY_BASE_MS = 2000;
export const RETRY_CAP_MS = 30_000;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

type FontFamily = { familyName: string; source: FontFamilySource; fonts: readonly Font[] };
type Font = { weight: number; url: string; hash: string };
type FontData = { type: 'manifest' } | { type: 'base' } | { type: 'chunk'; id: number };

type FontPathEntry = { url: string; hash: string };
let fontPaths = new Map<string, FontPathEntry>();

function fontKey(family: string, weight: number): string {
  return `${family}:${weight}`;
}

export type RetryChain = { gen: number; attempt: number };

function purgePrefixesFor(fontKeys: readonly string[]): string[] {
  return fontKeys.flatMap((fk) => [`manifest:${fk}:`, `base:${fk}:`, `chunk:${fk}:`]);
}

function purgeByPrefix(container: Map<string, unknown> | Set<string>, prefixes: readonly string[]): void {
  for (const key of container.keys()) {
    if (prefixes.some((prefix) => key.startsWith(prefix))) container.delete(key);
  }
}

export class FontLoaderState {
  readonly loaded = new Set<string>();
  readonly loading = new Map<string, Promise<boolean>>();
  readonly retryScheduled = new Map<string, RetryChain>();
  readonly generations = new Map<string, number>();

  generationOf(fontKey: string): number {
    return this.generations.get(fontKey) ?? 0;
  }

  isStale(fontKey: string, dispatchGen: number): boolean {
    return this.generationOf(fontKey) !== dispatchGen;
  }

  purge(fontKeys: readonly string[]): void {
    if (fontKeys.length === 0) return;

    for (const fk of fontKeys) this.generations.set(fk, this.generationOf(fk) + 1);

    const prefixes = purgePrefixesFor(fontKeys);
    purgeByPrefix(this.loaded, prefixes);
    purgeByPrefix(this.loading, prefixes);
    purgeByPrefix(this.retryScheduled, prefixes);
  }
}

const state = new FontLoaderState();

export async function loadOnce(loaderState: FontLoaderState, key: string, fn: () => Promise<boolean>): Promise<boolean> {
  // Invariant: `loaded` never claims a key the Rust registry lacks, so this early return never skips a commit the registry is
  // missing (which would otherwise ping-pong through the event queue via dispatch→commit→fan-out→re-resolve→re-emit); loadFonts
  // purges host state synchronously in the same block that replaces the snapshot, keeping the two in lockstep.
  if (loaderState.loaded.has(key)) return true;

  const existing = loaderState.loading.get(key);
  if (existing) return existing;

  const promise = fn();
  loaderState.loading.set(key, promise);

  try {
    return await promise;
  } finally {
    if (loaderState.loading.get(key) === promise) loaderState.loading.delete(key);
  }
}

export function scheduleRetry(loaderState: FontLoaderState, key: string, gen: number, fn: () => Promise<void>): void {
  const existing = loaderState.retryScheduled.get(key);
  if (existing && existing.gen === gen) return;

  loaderState.retryScheduled.set(key, { gen, attempt: 1 });
  retryStep(loaderState, key, gen, 1, fn);
}

function retryStep(loaderState: FontLoaderState, key: string, gen: number, attempt: number, fn: () => Promise<void>): void {
  if (attempt > RETRY_MAX_ATTEMPTS) {
    if (loaderState.retryScheduled.get(key)?.gen === gen) loaderState.retryScheduled.delete(key);
    return;
  }

  const delayMs = Math.min(RETRY_BASE_MS * Math.pow(2, attempt - 1), RETRY_CAP_MS);
  setTimeout(() => {
    void retryFire(loaderState, key, gen, attempt, fn);
  }, delayMs);
}

async function retryFire(loaderState: FontLoaderState, key: string, gen: number, attempt: number, fn: () => Promise<void>): Promise<void> {
  const chain = loaderState.retryScheduled.get(key);
  if (!chain || chain.gen !== gen) return;

  if (loaderState.loaded.has(key)) {
    loaderState.retryScheduled.delete(key);
    return;
  }

  loaderState.retryScheduled.set(key, { gen, attempt });

  const inflight = loaderState.loading.get(key);
  if (inflight) {
    await inflight.catch(() => false);
    if (loaderState.loaded.has(key)) {
      if (loaderState.retryScheduled.get(key)?.gen === gen) loaderState.retryScheduled.delete(key);
    } else {
      retryStep(loaderState, key, gen, attempt + 1, fn);
    }
    return;
  }

  try {
    await fn();
    if (loaderState.retryScheduled.get(key)?.gen === gen) loaderState.retryScheduled.delete(key);
  } catch {
    retryStep(loaderState, key, gen, attempt + 1, fn);
  }
}

export function loadFonts(families: readonly FontFamily[]): void {
  const next = new Map<string, FontPathEntry>();
  for (const family of families) {
    for (const font of family.fonts) {
      next.set(fontKey(family.familyName, font.weight), { url: font.url, hash: font.hash });
    }
  }

  const changedKeys = new Set<string>();
  for (const key of next.keys()) {
    if (fontPaths.get(key)?.hash !== next.get(key)?.hash) changedKeys.add(key);
  }
  for (const key of fontPaths.keys()) {
    if (!next.has(key)) changedKeys.add(key);
  }
  const changed = [...changedKeys];

  fontPaths = next;
  state.purge(changed);
  preloadQueue.purge(purgePrefixesFor(changed));

  wasm.set_fonts(
    families.map((family) => ({
      name: family.familyName,
      source: family.source,
      weights: family.fonts.map((font) => ({
        value: font.weight,
        hash: font.hash,
      })),
    })),
  );

  for (const editor of snapshot()) {
    editor.enqueue({ type: 'system', event: { type: 'fonts_changed' } });
  }
}

let cachePromise: Promise<Cache> | null = null;

function getCache(): Promise<Cache> {
  cachePromise ??= caches.open(CACHE_NAME);
  return cachePromise;
}

async function getOrFetch(url: string): Promise<Uint8Array> {
  const cache = await getCache();
  const cached = await cache.match(url);
  if (cached) return new Uint8Array(await cached.arrayBuffer());

  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to fetch: ${url}`);
  await cache.put(url, response.clone());

  return new Uint8Array(await response.arrayBuffer());
}

type PreloadItem = {
  key: string;
  priority: number;
  fn: () => Promise<void>;
  promise: Promise<void>;
  resolve: () => void;
  reject: (err: unknown) => void;
};

export class PreloadQueue {
  #pending: PreloadItem[] = [];
  #inflight = 0;
  #queued = new Map<string, PreloadItem>();

  #flush(): void {
    while (this.#inflight < PRELOAD_CONCURRENCY && this.#pending.length > 0) {
      const item = this.#pending.shift();
      if (!item) break;

      if (state.loaded.has(item.key)) {
        if (this.#queued.get(item.key) === item) this.#queued.delete(item.key);
        item.resolve();
        continue;
      }

      this.#inflight++;
      item.fn().then(
        () => {
          if (this.#queued.get(item.key) === item) this.#queued.delete(item.key);
          item.resolve();
          this.#inflight--;
          this.#flush();
        },
        (err) => {
          if (this.#queued.get(item.key) === item) this.#queued.delete(item.key);
          item.reject(err);
          this.#inflight--;
          this.#flush();
        },
      );
    }
  }

  enqueue(key: string, priority: number, fn: () => Promise<void>): Promise<void> {
    if (state.loaded.has(key)) return Promise.resolve();

    const existing = this.#queued.get(key);
    if (existing) return existing.promise;

    const { promise, resolve, reject }: PromiseWithResolvers<void> = Promise.withResolvers();

    const item: PreloadItem = { key, priority, fn, promise, resolve, reject };
    let i = this.#pending.findIndex((p) => p.priority < priority);
    if (i === -1) i = this.#pending.length;
    this.#pending.splice(i, 0, item);

    this.#queued.set(key, item);
    this.#flush();

    return promise;
  }

  purge(prefixes: readonly string[]): void {
    const remaining: PreloadItem[] = [];
    for (const item of this.#pending) {
      if (prefixes.some((prefix) => item.key.startsWith(prefix))) {
        item.resolve();
      } else {
        remaining.push(item);
      }
    }
    this.#pending = remaining;

    purgeByPrefix(this.#queued, prefixes);
  }
}

const preloadQueue = new PreloadQueue();

function keyOf(family: string, weight: number, hash: string, fd: FontData): string {
  if (fd.type === 'manifest') return `manifest:${family}:${weight}:${hash}`;
  if (fd.type === 'base') return `base:${family}:${weight}:${hash}`;
  return `chunk:${family}:${weight}:${hash}:${fd.id}`;
}

function urlOf(baseUrl: string, fd: FontData): string {
  if (fd.type === 'manifest') return `${baseUrl}/manifest.v1`;
  if (fd.type === 'base') return `${baseUrl}/base`;
  return `${baseUrl}/chunks/${fd.id}`;
}

async function load(
  family: string,
  weight: number,
  dispatchHash: string,
  dispatchGen: number,
  fd: FontData,
  baseUrl: string,
  attempts: number,
): Promise<void> {
  const fk = fontKey(family, weight);
  const key = keyOf(family, weight, dispatchHash, fd);

  const committed = await loadOnce(state, key, async () => {
    let lastErr: unknown;
    for (let attempt = 1; attempt <= attempts; attempt++) {
      const url = urlOf(baseUrl, fd);
      try {
        const data = await getOrFetch(url);

        if (state.isStale(fk, dispatchGen)) return false;

        try {
          if (fd.type === 'manifest') {
            wasm.add_font_manifest(family, weight, data);
          } else if (fd.type === 'base') {
            wasm.add_font_base(family, weight, data);
          } else {
            wasm.add_font_chunk(family, weight, fd.id, data);
          }
        } catch (err) {
          const cache = await getCache();
          await cache.delete(url);
          throw err;
        }
        state.loaded.add(key);
        return true;
      } catch (err) {
        lastErr = err;
        if (attempt < attempts) {
          await sleep(LOAD_RETRY_BASE_MS * Math.pow(2, attempt - 1));
        }
      }
    }
    throw lastErr;
  });

  if (!committed) return;

  for (const target of snapshot()) {
    if (fd.type === 'manifest') {
      target.enqueue({ type: 'system', event: { type: 'font_manifest_loaded', family, weight } });
    } else if (fd.type === 'base') {
      target.enqueue({ type: 'system', event: { type: 'font_base_loaded', family, weight } });
    } else {
      target.enqueue({ type: 'system', event: { type: 'font_chunk_loaded', family, weight, chunk_id: fd.id } });
    }
  }
}

async function loadRequired(
  family: string,
  weight: number,
  dispatchHash: string,
  dispatchGen: number,
  fd: FontData,
  baseUrl: string,
): Promise<void> {
  try {
    await load(family, weight, dispatchHash, dispatchGen, fd, baseUrl, REQUIRED_LOAD_ATTEMPTS);
  } catch {
    scheduleRetry(state, keyOf(family, weight, dispatchHash, fd), dispatchGen, () =>
      load(family, weight, dispatchHash, dispatchGen, fd, baseUrl, REQUIRED_LOAD_ATTEMPTS),
    );
  }
}

export const fontDataMissingHandler: EditorEventListener<'font_data_missing'> = async (editor, { family, weight, required, prefetch }) => {
  const fk = fontKey(family, weight);
  const info = fontPaths.get(fk);
  if (!info) {
    console.warn(`No font path registered for ${family}:${weight}`);
    return;
  }

  const dispatchHash = info.hash;
  const dispatchGen = state.generationOf(fk);
  const baseUrl = `${info.url}/${dispatchHash}`;

  const manifestRequired = required.filter((fd): fd is Extract<FontData, { type: 'manifest' }> => fd.type === 'manifest');
  const baseRequired = required.filter((fd): fd is Extract<FontData, { type: 'base' }> => fd.type === 'base');
  const chunkRequired = required.filter((fd): fd is Extract<FontData, { type: 'chunk' }> => fd.type === 'chunk');

  await Promise.allSettled(manifestRequired.map((fd) => loadRequired(family, weight, dispatchHash, dispatchGen, fd, baseUrl)));
  await Promise.allSettled(baseRequired.map((fd) => loadRequired(family, weight, dispatchHash, dispatchGen, fd, baseUrl)));
  await Promise.allSettled(chunkRequired.map((fd) => loadRequired(family, weight, dispatchHash, dispatchGen, fd, baseUrl)));

  // Read-only viewers never type new codepoints, so speculative whole-family
  // prefetch only wastes reader bandwidth; visible text is covered by `required`.
  if (editor.readOnly) {
    return;
  }

  for (const fd of prefetch) {
    const priority = fd.type === 'manifest' ? -2 : fd.type === 'base' ? -1 : fd.id;
    preloadQueue.enqueue(keyOf(family, weight, dispatchHash, fd), priority, async () => {
      try {
        await load(family, weight, dispatchHash, dispatchGen, fd, baseUrl, PREFETCH_LOAD_ATTEMPTS);
      } catch {
        // best-effort
      }
    });
  }
};
