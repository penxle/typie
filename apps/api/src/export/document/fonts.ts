import fallbackFontFamilies from '@typie/editor/font/fallbacks.json' with { type: 'json' };
import type { Application } from '@typie/editor';

export type Font = {
  weight: number;
  url: string;
};

export type FontFamily = {
  familyName: string;
  fonts: Font[];
};

type FontManifest = {
  hash: string;
  chunk_count: number;
  chunk_map: string | null;
  chunk_map_sup?: number[];
};

const CDN_BASE = 'https://cdn.typie.net/editor/fonts';

const phantomFontFamilies = [
  { familyName: 'Noto (Phantom)', path: import.meta.resolve?.('@typie/editor/font/Noto-Phantom.bin') as string },
  { familyName: 'Noto Emoji (Phantom)', path: import.meta.resolve?.('@typie/editor/font/Noto-Phantom-Emoji.bin') as string },
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
const instanceLoaded = new WeakMap<Application, Set<string>>();
const instanceLoading = new WeakMap<Application, Map<string, Promise<void>>>();
const decodedChunkMaps = new Map<FontManifest, Uint8Array>();

function fetchFont(url: string): Promise<Uint8Array> {
  return fontData.resolve(url, async () => {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`Failed to fetch font: ${url}`);
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

function loadOnce(app: Application, key: string, fn: () => Promise<void>): Promise<void> {
  let loaded = instanceLoaded.get(app);
  if (!loaded) {
    loaded = new Set();
    instanceLoaded.set(app, loaded);
  }
  if (loaded.has(key)) return Promise.resolve();

  let loading = instanceLoading.get(app);
  if (!loading) {
    loading = new Map();
    instanceLoading.set(app, loading);
  }

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

async function loadBase(app: Application, family: string, font: Font): Promise<void> {
  await loadOnce(app, `base:${family}:${font.weight}`, async () => {
    const manifest = await fetchManifest(font.url);
    const buffer = await fetchFont(`${font.url}/${manifest.hash}/base.bin`);
    app.addFontBase(family, font.weight, buffer);
  });
}

async function loadChunks(app: Application, family: string, font: Font, codepoints: number[]): Promise<void> {
  const manifest = await fetchManifest(font.url);

  await Promise.allSettled(
    findChunkIndices(manifest, codepoints).map((idx) =>
      loadOnce(app, `chunk:${family}:${font.weight}:${idx}`, async () => {
        const buffer = await fetchFont(`${font.url}/${manifest.hash}/chunks/${idx}.bin`);
        app.addFontChunk(family, font.weight, buffer);
      }),
    ),
  );
}

export async function initFonts(app: Application): Promise<void> {
  await Promise.all(
    phantomFontFamilies.map(async ({ familyName, path }) => {
      const data = await Bun.file(new URL(path)).bytes();
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

export async function filterUncoveredCodepoints(font: Font, codepoints: number[]): Promise<number[]> {
  const manifest = await fetchManifest(font.url);
  return codepoints.filter((cp) => !hasCodepoint(manifest, cp));
}

export async function ensureRequiredFont(app: Application, family: string, font: Font, codepoints: number[]): Promise<void> {
  await loadBase(app, family, font);
  await loadChunks(app, family, font, codepoints);
}

export async function ensureRequiredFallbackFont(app: Application, weight: number, codepoints: number[]): Promise<void> {
  const tasks: { family: string; font: Font; codepoints: number[] }[] = [];
  let remaining = codepoints;

  for (const fallbackFontFamily of fallbackFontFamilies) {
    if (remaining.length === 0) break;

    const fallbackFont = fallbackFontFamily.fonts.find((f) => f.weight === weight);
    if (!fallbackFont) continue;

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
    }),
  );
}
