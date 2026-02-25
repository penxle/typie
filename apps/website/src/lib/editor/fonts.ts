import fallbackFontFamilies from '@typie/editor/font/fallbacks.json' with { type: 'json' };
import notoPhantomUrl from '@typie/editor/font/Noto-Phantom.bin?url';
import notoPhantomEmojiUrl from '@typie/editor/font/Noto-Phantom-Emoji.bin?url';
import type { Application } from '@typie/editor';

export type Font = { id: string; weight: number; subfamilyDisplayName?: string | null; url: string; state: string };
export type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };
type FontRef = { weight: number; url: string };

export function getRepresentativeFont(fonts: readonly Font[]): Font | null {
  const active = fonts.filter((f) => f.state === 'ACTIVE');
  if (active.length === 0) return null;
  return active.reduce((prev, curr) => {
    const prevDiff = Math.abs(prev.weight - 400);
    const currDiff = Math.abs(curr.weight - 400);
    if (currDiff < prevDiff) return curr;
    if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
    return prev;
  });
}
type FontManifest = {
  hash: string;
  chunk_count: number;
  chunk_map: string | null;
  chunk_map_sup?: number[];
};

const CDN_BASE = 'https://cdn.typie.net/editor/fonts';
const CACHE_NAME = 'typie-fonts';

const phantomFontFamilies = [
  { familyName: 'Noto (Phantom)', url: notoPhantomUrl },
  { familyName: 'Noto Emoji (Phantom)', url: notoPhantomEmojiUrl },
];

function createAsyncCache<T>() {
  const values = new Map<string, T>();
  const inflight = new Map<string, Promise<T>>();

  return {
    set(key: string, value: T) {
      values.set(key, value);
    },
    resolve(key: string, fn: () => Promise<T>): Promise<T> {
      const cached = values.get(key);
      if (cached !== undefined) return Promise.resolve(cached);

      let promise = inflight.get(key);
      if (!promise) {
        promise = fn().then(
          (result) => {
            if (result != null) values.set(key, result);
            inflight.delete(key);
            return result;
          },
          (err) => {
            inflight.delete(key);
            throw err;
          },
        );
        inflight.set(key, promise);
      }
      return promise;
    },
  };
}

const fontData = createAsyncCache<Uint8Array>();
const fontManifests = createAsyncCache<FontManifest>();
const loaded = new Set<string>();
const loading = new Map<string, Promise<void>>();
const decodedChunkMaps = new Map<FontManifest, Uint8Array>();

const PRELOAD_CONCURRENCY = 4;

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

let cachePromise: Promise<Cache> | null = null;

function getCache(): Promise<Cache> {
  cachePromise ??= caches.open(CACHE_NAME);
  return cachePromise;
}

function fetchFont(url: string): Promise<Uint8Array> {
  return fontData.resolve(url, async () => {
    const cache = await getCache();
    const cached = await cache.match(url);
    if (cached) return new Uint8Array(await cached.arrayBuffer());

    const response = await fetch(url);
    if (!response.ok) throw new Error(`Failed to fetch font: ${url}`);
    await cache.put(url, response.clone());
    return new Uint8Array(await response.arrayBuffer());
  });
}

function fetchManifest(url: string): Promise<FontManifest> {
  return fontManifests.resolve(url, async () => {
    const response = await fetch(`${url}/manifest.json`);
    if (!response.ok) throw new Error(`Failed to fetch manifest: ${url}`);
    return response.json();
  });
}

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

function decodeChunkMap(manifest: FontManifest): Uint8Array | null {
  if (!manifest.chunk_map) return null;
  let map = decodedChunkMaps.get(manifest);
  if (!map) {
    map = Uint8Array.fromBase64(manifest.chunk_map);
    decodedChunkMaps.set(manifest, map);
  }
  return map;
}

function lookupChunkIndex(data: Uint8Array, sup: number[] | undefined, cp: number): number {
  if (cp <= 0xff_ff) {
    const l2 = data[cp >>> 8];
    if (l2 === 0xff) return -1;
    const chunk = data[256 + l2 * 256 + (cp & 0xff)];
    return chunk === 0xff ? -1 : chunk;
  }
  if (sup) {
    let lo = 0;
    let hi = sup.length / 2 - 1;
    while (lo <= hi) {
      const mid = (lo + hi) >>> 1;
      const key = sup[mid * 2];
      if (cp < key) hi = mid - 1;
      else if (cp > key) lo = mid + 1;
      else return sup[mid * 2 + 1];
    }
  }
  return -1;
}

function hasCodepoint(manifest: FontManifest, cp: number): boolean {
  const data = decodeChunkMap(manifest);
  if (!data) return false;
  return lookupChunkIndex(data, manifest.chunk_map_sup, cp) >= 0;
}

function findChunkIndices(manifest: FontManifest, codepoints: number[]): number[] {
  const data = decodeChunkMap(manifest);
  if (!data) return [];
  const indices = new Set<number>();
  for (const cp of codepoints) {
    const idx = lookupChunkIndex(data, manifest.chunk_map_sup, cp);
    if (idx >= 0) indices.add(idx);
  }
  return [...indices];
}

async function loadBase(app: Application, family: string, font: FontRef): Promise<void> {
  await loadOnce(`base:${family}:${font.weight}`, async () => {
    const manifest = await fetchManifest(font.url);
    const buffer = await fetchFont(`${font.url}/${manifest.hash}/base.bin`);
    app.addFontBase(family, font.weight, buffer);
  });
}

async function loadChunks(app: Application, family: string, font: FontRef, codepoints: number[]): Promise<void> {
  const manifest = await fetchManifest(font.url);

  await Promise.allSettled(
    findChunkIndices(manifest, codepoints).map((idx) =>
      loadOnce(`chunk:${family}:${font.weight}:${idx}`, async () => {
        const buffer = await fetchFont(`${font.url}/${manifest.hash}/chunks/${idx}.bin`);
        app.addFontChunk(family, font.weight, buffer);
      }),
    ),
  );
}

export async function initFonts(app: Application): Promise<void> {
  await Promise.all(
    phantomFontFamilies.map(async ({ familyName, url }) => {
      const response = await fetch(url);
      const data = new Uint8Array(await response.arrayBuffer());
      app.addFontBase(familyName, 400, data);
    }),
  );

  for (const fontFamily of fallbackFontFamilies) {
    for (const font of fontFamily.fonts) {
      fontManifests.set(`${CDN_BASE}/${font.path}`, font);
    }
  }

  app.setFallbackFonts([...fallbackFontFamilies.map((f) => f.familyName), ...phantomFontFamilies.map((f) => f.familyName)]);
}

export async function filterUncoveredCodepoints(font: FontRef, codepoints: number[]): Promise<number[]> {
  const manifest = await fetchManifest(font.url);
  return codepoints.filter((cp) => !hasCodepoint(manifest, cp));
}

export async function ensureRequiredFont(app: Application, family: string, font: FontRef, codepoints: number[]): Promise<void> {
  await loadBase(app, family, font);
  await loadChunks(app, family, font, codepoints);
}

export async function preloadRemainingChunks(app: Application, family: string, font: FontRef): Promise<void> {
  try {
    const manifest = await fetchManifest(font.url);

    for (let i = manifest.chunk_count - 1; i >= 0; i--) {
      const key = `chunk:${family}:${font.weight}:${i}`;
      if (!loaded.has(key)) {
        preloadQueue.enqueue(key, i / manifest.chunk_count, async () => {
          try {
            await loadOnce(key, async () => {
              const buffer = await fetchFont(`${font.url}/${manifest.hash}/chunks/${i}.bin`);
              app.addFontChunk(family, font.weight, buffer);
            });
          } catch {
            // best-effort: silently ignore preload failures
          }
        });
      }
    }
  } catch {
    // best-effort
  }
}

export async function ensureRequiredFallbackFont(app: Application, weight: number, codepoints: number[]): Promise<void> {
  const tasks: { family: string; font: { weight: number; url: string }; codepoints: number[] }[] = [];
  let remaining = codepoints;

  for (const fallbackFontFamily of fallbackFontFamilies) {
    if (remaining.length === 0) break;

    if (fallbackFontFamily.fonts.length === 0) continue;
    const fallbackFont = fallbackFontFamily.fonts.reduce((prev, curr) => {
      const prevDiff = Math.abs(prev.weight - weight);
      const currDiff = Math.abs(curr.weight - weight);
      if (currDiff < prevDiff) return curr;
      if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
      return prev;
    });

    const covered = remaining.filter((cp) => hasCodepoint(fallbackFont, cp));
    if (covered.length === 0) continue;

    tasks.push({
      family: fallbackFontFamily.familyName,
      font: { weight: fallbackFont.weight, url: `${CDN_BASE}/${fallbackFont.path}` },
      codepoints: covered,
    });

    const coveredSet = new Set(covered);
    remaining = remaining.filter((cp) => !coveredSet.has(cp));
  }

  await Promise.all(
    tasks.map(async ({ family, font, codepoints }) => {
      await loadBase(app, family, font);
      await loadChunks(app, family, font, codepoints);
      app.setFallbackFonts([...fallbackFontFamilies.map((f) => f.familyName), ...phantomFontFamilies.map((f) => f.familyName)]);
    }),
  );
}
