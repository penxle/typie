import fallbackManifestUrl from '@typie/assets/fallbacks.bin?url';
import fallbackFontFamilies from '@typie/assets/fallbacks.json' with { type: 'json' };
import notoPhantomUrl from '@typie/assets/Noto-Phantom.bin?url';
import notoPhantomEmojiUrl from '@typie/assets/Noto-Phantom-Emoji.bin?url';
import { wasm } from '$lib/wasm-ffi.svelte';
import type { Editor, FontData } from '@typie/editor-ffi/browser';

const CDN_BASE = 'https://cdn.typie.net/editor/fonts';
const CACHE_NAME = 'typie-fonts';
const PRELOAD_CONCURRENCY = 4;

const phantomFontFamilies = [
  { familyName: 'Noto (Phantom)', url: notoPhantomUrl },
  { familyName: 'Noto Emoji (Phantom)', url: notoPhantomEmojiUrl },
];

const fontPaths = new Map<string, { path: string; hash: string }>();

function fontKey(family: string, weight: number): string {
  return `${family}:${weight}`;
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

async function fetchBinary(url: string): Promise<Uint8Array> {
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

export async function initFonts(): Promise<void> {
  await Promise.all(
    phantomFontFamilies.map(async ({ familyName, url }) => {
      const response = await fetch(url);
      const data = new Uint8Array(await response.arrayBuffer());
      wasm.load_font_base(familyName, 400, data);
    }),
  );

  wasm.set_phantom_font_families(phantomFontFamilies.map((v) => v.familyName));

  // Load fallback manifests (binary → Rust)
  const response = await fetch(fallbackManifestUrl);
  const fallbackData = new Uint8Array(await response.arrayBuffer());
  wasm.load_fallback_font_manifests(fallbackData);

  // Populate host-side path/hash registry from JSON (for CDN URL construction)
  for (const family of fallbackFontFamilies) {
    for (const font of family.fonts) {
      fontPaths.set(fontKey(family.familyName, font.weight), {
        path: font.path,
        hash: font.hash,
      });
    }
  }
}

export async function handleFontManifestMissing(editor: Editor, family: string, weight: number, fontPath: string): Promise<void> {
  const [manifestBin, hashJson] = await Promise.all([
    fetchBinary(`${CDN_BASE}/${fontPath}/manifest.bin`),
    fetch(`${CDN_BASE}/${fontPath}/hash.json`).then((r) => r.json() as Promise<{ hash: string }>),
  ]);

  fontPaths.set(fontKey(family, weight), { path: fontPath, hash: hashJson.hash });

  wasm.load_font_manifest(family, weight, manifestBin);
  editor.enqueue({ type: 'system', value: { type: 'font_manifest_loaded', value: { family, weight } } });
}

export async function handleFontDataMissing(
  editor: Editor,
  family: string,
  weight: number,
  required: FontData[],
  prefetch: FontData[],
): Promise<void> {
  const info = fontPaths.get(fontKey(family, weight));
  if (!info) {
    console.warn(`No font path registered for ${family}:${weight}`);
    return;
  }

  const baseUrl = `${CDN_BASE}/${info.path}/${info.hash}`;
  const enqueueBase = () => editor.enqueue({ type: 'system', value: { type: 'font_base_loaded', value: { family, weight } } });
  const enqueueChunk = () => editor.enqueue({ type: 'system', value: { type: 'font_chunk_loaded', value: { family, weight } } });

  const base = required.find((item) => item.type === 'base');
  const chunks = required.filter((item) => item.type === 'chunk');

  if (base) {
    await loadOnce(`base:${family}:${weight}`, async () => {
      const data = await fetchBinary(`${baseUrl}/base.bin`);
      wasm.load_font_base(family, weight, data);
      enqueueBase();
    });
  }

  const loadChunk = (idx: number) =>
    loadOnce(`chunk:${family}:${weight}:${idx}`, async () => {
      const data = await fetchBinary(`${baseUrl}/chunks/${idx}.bin`);
      wasm.load_font_chunk(family, weight, data);
      enqueueChunk();
    });

  await Promise.all(chunks.map((item) => loadChunk(item.value)));

  for (const item of prefetch) {
    if (item.type === 'chunk') {
      const idx = item.value;
      preloadQueue.enqueue(`chunk:${family}:${weight}:${idx}`, idx, async () => {
        try {
          await loadChunk(idx);
        } catch {
          // best-effort
        }
      });
    }
  }
}
