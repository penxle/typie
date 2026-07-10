import { wasm } from '$lib/wasm-ffi.svelte';
import { snapshot } from './registry';
import type { FontFamilySource } from '$mearie';
import type { EditorEventListener } from './types';

const CACHE_NAME = 'typie-fonts';
const PRELOAD_CONCURRENCY = 4;
const REQUIRED_LOAD_ATTEMPTS = 3;
const PREFETCH_LOAD_ATTEMPTS = 1;
const LOAD_RETRY_BASE_MS = 200;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

type FontFamily = { familyName: string; source: FontFamilySource; fonts: readonly Font[] };
type Font = { weight: number; url: string; hash: string; chunks: unknown };
type FontData = { type: 'base' } | { type: 'chunk'; id: number };

type FontPathEntry = { url: string; hash: string };
const fontPaths = new Map<string, FontPathEntry>();

function fontKey(family: string, weight: number): string {
  return `${family}:${weight}`;
}

export function loadFonts(families: readonly FontFamily[]): void {
  for (const family of families) {
    for (const font of family.fonts) {
      fontPaths.set(fontKey(family.familyName, font.weight), { url: font.url, hash: font.hash });
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

  for (const editor of snapshot()) {
    editor.enqueue({ type: 'system', event: { type: 'fonts_changed' } });
  }
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
  attempts: number,
): Promise<void> {
  await loadOnce(keyOf(family, weight, fd), async () => {
    let lastErr: unknown;
    for (let attempt = 1; attempt <= attempts; attempt++) {
      try {
        const data = await getOrFetch(urlOf(baseUrl, fd));
        if (fd.type === 'base') {
          wasm.add_font_base(family, weight, data);
        } else {
          wasm.add_font_chunk(family, weight, fd.id, data);
        }
        return;
      } catch (err) {
        lastErr = err;
        if (attempt < attempts) {
          await sleep(LOAD_RETRY_BASE_MS * Math.pow(2, attempt - 1));
        }
      }
    }
    throw lastErr;
  });

  if (fd.type === 'base') {
    editor.enqueue({ type: 'system', event: { type: 'font_base_loaded', family, weight } });
  } else {
    editor.enqueue({ type: 'system', event: { type: 'font_chunk_loaded', family, weight, chunk_id: fd.id } });
  }
}

export const fontDataMissingHandler: EditorEventListener<'font_data_missing'> = async (editor, { family, weight, required, prefetch }) => {
  const info = fontPaths.get(fontKey(family, weight));
  if (!info) {
    console.warn(`No font path registered for ${family}:${weight}`);
    return;
  }
  const baseUrl = `${info.url}/${info.hash}`;

  const baseRequired = required.filter((fd): fd is Extract<FontData, { type: 'base' }> => fd.type === 'base');
  const chunkRequired = required.filter((fd): fd is Extract<FontData, { type: 'chunk' }> => fd.type === 'chunk');

  await Promise.allSettled(baseRequired.map((fd) => load(editor, family, weight, fd, baseUrl, REQUIRED_LOAD_ATTEMPTS)));
  await Promise.allSettled(chunkRequired.map((fd) => load(editor, family, weight, fd, baseUrl, REQUIRED_LOAD_ATTEMPTS)));

  for (const fd of prefetch) {
    const priority = fd.type === 'base' ? -1 : fd.id;
    preloadQueue.enqueue(keyOf(family, weight, fd), priority, async () => {
      try {
        await load(editor, family, weight, fd, baseUrl, PREFETCH_LOAD_ATTEMPTS);
      } catch {
        // best-effort
      }
    });
  }
};
