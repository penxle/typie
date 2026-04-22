import { wasm } from '$lib/wasm-ffi.svelte';
import type { FontFamilySource } from '$mearie';
import type { EditorEventListener } from './types';

const CDN_BASE = 'https://cdn.typie.net/editor/fonts';
const CACHE_NAME = 'typie-fonts';
const PRELOAD_CONCURRENCY = 4;

type FontFamily = { familyName: string; source: FontFamilySource; fonts: readonly Font[] };
type Font = { weight: number; path: string; hash: string; chunks: unknown };
type FontData = { type: 'base' } | { type: 'chunk'; id: number };

type FontPathEntry = { path: string; hash: string };
const fontPaths = new Map<string, FontPathEntry>();

function fontKey(family: string, weight: number): string {
  return `${family}:${weight}`;
}

export function loadFonts(families: readonly FontFamily[]): void {
  for (const family of families) {
    for (const font of family.fonts) {
      fontPaths.set(fontKey(family.familyName, font.weight), { path: font.path, hash: font.hash });
    }
  }

  wasm.set_fonts(
    families.map((family) => ({
      name: family.familyName,
      source: family.source,
      weights: family.fonts.map((font) => ({
        value: font.weight,
        hash: font.hash,
        chunks: font.chunks as never,
      })),
    })),
  );
}

const loaded = new Set<string>();
const loading = new Map<string, Promise<void>>();

function loadOnce(key: string, fn: () => Promise<void>): Promise<void> {
  if (loaded.has(key)) return Promise.resolve();

  let promise = loading.get(key);
  if (!promise) {
    promise = fn().then(
      () => {
        loaded.add(key);
        loading.delete(key);
      },
      (err) => {
        loading.delete(key);
        throw err;
      },
    );
    loading.set(key, promise);
  }
  return promise;
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
  resolve: () => void;
  reject: (err: unknown) => void;
};

class PreloadQueue {
  #pending: PreloadItem[] = [];
  #inflight = 0;
  #promises = new Map<string, Promise<void>>();

  enqueue(key: string, priority: number, fn: () => Promise<void>): Promise<void> {
    if (loaded.has(key)) return Promise.resolve();

    const existing = this.#promises.get(key);
    if (existing) return existing;

    const promise = new Promise<void>((resolve, reject) => {
      const item: PreloadItem = { key, priority, fn, resolve, reject };
      let i = this.#pending.findIndex((p) => p.priority < priority);
      if (i === -1) i = this.#pending.length;
      this.#pending.splice(i, 0, item);
    });

    this.#promises.set(key, promise);
    this.#flush();

    return promise;
  }

  #flush(): void {
    while (this.#inflight < PRELOAD_CONCURRENCY && this.#pending.length > 0) {
      const item = this.#pending.shift();
      if (!item) break;

      if (loaded.has(item.key)) {
        this.#promises.delete(item.key);
        item.resolve();
        continue;
      }

      this.#inflight++;
      item.fn().then(
        () => {
          this.#promises.delete(item.key);
          item.resolve();
          this.#inflight--;
          this.#flush();
        },
        (err) => {
          this.#promises.delete(item.key);
          item.reject(err);
          this.#inflight--;
          this.#flush();
        },
      );
    }
  }
}

const preloadQueue = new PreloadQueue();

function keyOf(family: string, weight: number, fd: FontData): string {
  return fd.type === 'base' ? `base:${family}:${weight}` : `chunk:${family}:${weight}:${fd.id}`;
}

function urlOf(baseUrl: string, fd: FontData): string {
  return fd.type === 'base' ? `${baseUrl}/base` : `${baseUrl}/chunks/${fd.id}`;
}

async function load(
  editor: Parameters<EditorEventListener<'font_data_missing'>>[0],
  family: string,
  weight: number,
  fd: FontData,
  baseUrl: string,
): Promise<void> {
  await loadOnce(keyOf(family, weight, fd), async () => {
    const data = await getOrFetch(urlOf(baseUrl, fd));
    if (fd.type === 'base') {
      wasm.add_font_base(family, weight, data);
      editor.enqueue({ type: 'system', event: { type: 'font_base_loaded', family, weight } });
    } else {
      wasm.add_font_chunk(family, weight, fd.id, data);
      editor.enqueue({ type: 'system', event: { type: 'font_chunk_loaded', family, weight, chunk_id: fd.id } });
    }
  });
}

export const fontDataMissingHandler: EditorEventListener<'font_data_missing'> = async (editor, { family, weight, required, prefetch }) => {
  const info = fontPaths.get(fontKey(family, weight));
  if (!info) {
    console.warn(`No font path registered for ${family}:${weight}`);
    return;
  }
  const baseUrl = `${CDN_BASE}/${info.path}/${info.hash}`;

  await Promise.allSettled(required.map((fd) => load(editor, family, weight, fd, baseUrl)));

  for (const fd of prefetch) {
    const priority = fd.type === 'base' ? -1 : fd.id;
    preloadQueue.enqueue(keyOf(family, weight, fd), priority, async () => {
      try {
        await load(editor, family, weight, fd, baseUrl);
      } catch {
        // best-effort
      }
    });
  }
};
