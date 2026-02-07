import fontsManifest from '@typie/editor/pkg/fonts.json';
import type { Application } from '@typie/editor';

type FontVariant = { weight: number; path: string };
type Font = { family: string; variants: FontVariant[] };
type FallbackFont = Font & { priority: number };
type FontManifest = {
  hash: string;
  chunk_count: number;
  chunk_map: string | null;
  chunk_map_sup?: number[];
};

const CDN_BASE = 'https://cdn.typie.net/editor/fonts';
const manifest = fontsManifest as Record<string, FontManifest>;
const decodedMaps = new Map<FontManifest, Uint8Array>();

const DEFAULT_FONTS: Font[] = [
  {
    family: 'Pretendard',
    variants: [
      { weight: 100, path: 'Pretendard-Thin' },
      { weight: 200, path: 'Pretendard-ExtraLight' },
      { weight: 300, path: 'Pretendard-Light' },
      { weight: 400, path: 'Pretendard-Regular' },
      { weight: 500, path: 'Pretendard-Medium' },
      { weight: 600, path: 'Pretendard-SemiBold' },
      { weight: 700, path: 'Pretendard-Bold' },
      { weight: 800, path: 'Pretendard-ExtraBold' },
      { weight: 900, path: 'Pretendard-Black' },
    ],
  },
  {
    family: 'KoPubWorldDotum',
    variants: [
      { weight: 300, path: 'KoPubWorldDotum-Light' },
      { weight: 500, path: 'KoPubWorldDotum-Medium' },
      { weight: 700, path: 'KoPubWorldDotum-Bold' },
    ],
  },
  {
    family: 'NanumBarunGothic',
    variants: [
      { weight: 200, path: 'NanumBarunGothic-UltraLight' },
      { weight: 300, path: 'NanumBarunGothic-Light' },
      { weight: 400, path: 'NanumBarunGothic-Regular' },
      { weight: 700, path: 'NanumBarunGothic-Bold' },
    ],
  },
  { family: 'RIDIBatang', variants: [{ weight: 400, path: 'RIDIBatang-Regular' }] },
  {
    family: 'KoPubWorldBatang',
    variants: [
      { weight: 300, path: 'KoPubWorldBatang-Light' },
      { weight: 500, path: 'KoPubWorldBatang-Medium' },
      { weight: 700, path: 'KoPubWorldBatang-Bold' },
    ],
  },
  {
    family: 'NanumMyeongjo',
    variants: [
      { weight: 400, path: 'NanumMyeongjo-Regular' },
      { weight: 700, path: 'NanumMyeongjo-Bold' },
      { weight: 800, path: 'NanumMyeongjo-ExtraBold' },
    ],
  },
];

const FALLBACK_FONTS: FallbackFont[] = [
  { family: 'Pretendard (Fallback)', priority: 100, variants: [{ weight: 400, path: 'Pretendard-Regular' }] },
  {
    family: 'Noto Sans JP',
    priority: 200,
    variants: [
      { weight: 400, path: 'NotoSansJP-Regular' },
      { weight: 700, path: 'NotoSansJP-Bold' },
    ],
  },
  {
    family: 'Noto Sans SC',
    priority: 300,
    variants: [
      { weight: 400, path: 'NotoSansSC-Regular' },
      { weight: 700, path: 'NotoSansSC-Bold' },
    ],
  },
  { family: 'NotoColorEmoji', priority: 400, variants: [{ weight: 400, path: 'NotoColorEmoji' }] },
  { family: 'Noto (Phantom)', priority: 65_534, variants: [{ weight: 400, path: 'Noto-Phantom' }] },
  { family: 'Noto Emoji (Phantom)', priority: 65_535, variants: [{ weight: 400, path: 'Noto-Phantom-Emoji' }] },
];

const ALL_FONTS: Font[] = [...DEFAULT_FONTS, ...FALLBACK_FONTS];

const fontDataCache = new Map<string, Uint8Array>();
const loaded = new Set<string>();
const pending = new Map<string, Promise<void>>();
const fetching = new Map<string, Promise<Uint8Array>>();

async function fetchFont(url: string): Promise<Uint8Array> {
  const cached = fontDataCache.get(url);
  if (cached) return cached;

  const inflight = fetching.get(url);
  if (inflight) return inflight;

  const promise = (async () => {
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Failed to fetch font: ${url}`);
      const data = new Uint8Array(await response.arrayBuffer());
      fontDataCache.set(url, data);
      return data;
    } finally {
      fetching.delete(url);
    }
  })();

  fetching.set(url, promise);
  return promise;
}

async function loadOnce(key: string, fn: () => Promise<void>): Promise<void> {
  if (loaded.has(key)) return;

  const existing = pending.get(key);
  if (existing) return existing;

  const promise = (async () => {
    await fn();
    loaded.add(key);
  })();

  pending.set(key, promise);
  try {
    await promise;
  } finally {
    pending.delete(key);
  }
}

function getChunkMap(fm: FontManifest): Uint8Array | null {
  if (!fm.chunk_map) return null;
  let map = decodedMaps.get(fm);
  if (!map) {
    const binary = atob(fm.chunk_map);
    map = Uint8Array.from(binary, (c) => c.codePointAt(0) ?? 0);
    decodedMaps.set(fm, map);
  }
  return map;
}

function findSupplementaryChunk(sup: number[], cp: number): number {
  let lo = 0;
  let hi = sup.length / 2 - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1;
    const key = sup[mid * 2];
    if (cp < key) hi = mid - 1;
    else if (cp > key) lo = mid + 1;
    else return sup[mid * 2 + 1];
  }
  return -1;
}

function findChunkIndices(fm: FontManifest, codepoints: number[]): number[] {
  const data = getChunkMap(fm);
  if (!data) return [];
  const indices = new Set<number>();
  for (const cp of codepoints) {
    if (cp <= 0xff_ff) {
      const l2Idx = data[cp >>> 8];
      if (l2Idx === 0xff) continue;
      const chunk = data[256 + l2Idx * 256 + (cp & 0xff)];
      if (chunk !== 0xff) indices.add(chunk);
    } else if (fm.chunk_map_sup) {
      const idx = findSupplementaryChunk(fm.chunk_map_sup, cp);
      if (idx >= 0) indices.add(idx);
    }
  }
  return [...indices];
}

export async function ensureAllFontBases(app: Application): Promise<void> {
  const promises: Promise<void>[] = [];

  for (const config of ALL_FONTS) {
    for (const variant of config.variants) {
      const fm = manifest[variant.path];
      if (!fm) continue;

      promises.push(
        loadOnce(`base:${config.family}:${variant.weight}`, async () => {
          const buffer = await fetchFont(`${CDN_BASE}/${variant.path}/${fm.hash}/base.bin`);
          app.addFontBase(config.family, variant.weight, buffer);
        }),
      );
    }
  }

  await Promise.allSettled(promises);

  const fallbacks = FALLBACK_FONTS.map((c) => ({ family: c.family, priority: c.priority }));
  fallbacks.sort((a, b) => a.priority - b.priority);
  app.setFallbackFonts(fallbacks.map((f) => f.family));
}

async function loadChunks(app: Application, configs: Font[], codepoints: number[]): Promise<void> {
  const promises: Promise<void>[] = [];

  for (const config of configs) {
    for (const variant of config.variants) {
      const fm = manifest[variant.path];
      if (!fm) continue;

      for (const idx of findChunkIndices(fm, codepoints)) {
        promises.push(
          loadOnce(`chunk:${config.family}:${variant.weight}:${idx}`, async () => {
            const buffer = await fetchFont(`${CDN_BASE}/${variant.path}/${fm.hash}/chunks/${idx}.bin`);
            app.addFontChunk(config.family, variant.weight, buffer);
          }),
        );
      }
    }
  }

  await Promise.allSettled(promises);
}

export async function ensureRequiredFont(app: Application, family: string, weight: number, codepoints: number[]): Promise<void> {
  const config = DEFAULT_FONTS.find((c) => c.family === family);
  const variant = config?.variants.find((v) => v.weight === weight);
  if (!variant) return;

  await loadChunks(app, [{ family, variants: [variant] }], codepoints);
}

export async function ensureRequiredFallbackFont(app: Application, codepoints: number[]): Promise<void> {
  await loadChunks(app, FALLBACK_FONTS, codepoints);
}
