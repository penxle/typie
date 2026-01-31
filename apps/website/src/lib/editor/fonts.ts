import type { Application } from '@typie/editor';
import type { WritingSystem } from './types';

export type FontInfo = {
  family: string;
  weight: number;
  file: string;
};

const PHANTOM_FONTS: FontInfo[] = [{ family: 'Noto-Phantom', weight: 400, file: 'Noto-Phantom.ttf' }];

export const DEFAULT_FONTS: FontInfo[] = [
  { family: 'Pretendard', weight: 100, file: 'Pretendard-Thin.ttf' },
  { family: 'Pretendard', weight: 200, file: 'Pretendard-ExtraLight.ttf' },
  { family: 'Pretendard', weight: 300, file: 'Pretendard-Light.ttf' },
  { family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.ttf' },
  { family: 'Pretendard', weight: 500, file: 'Pretendard-Medium.ttf' },
  { family: 'Pretendard', weight: 600, file: 'Pretendard-SemiBold.ttf' },
  { family: 'Pretendard', weight: 700, file: 'Pretendard-Bold.ttf' },
  { family: 'Pretendard', weight: 800, file: 'Pretendard-ExtraBold.ttf' },
  { family: 'Pretendard', weight: 900, file: 'Pretendard-Black.ttf' },
  { family: 'KoPubWorldDotum', weight: 300, file: 'KoPubWorld Dotum Light.ttf' },
  { family: 'KoPubWorldDotum', weight: 500, file: 'KoPubWorld Dotum Medium.ttf' },
  { family: 'KoPubWorldDotum', weight: 700, file: 'KoPubWorld Dotum Bold.ttf' },
  { family: 'NanumBarunGothic', weight: 200, file: 'NanumBarunGothicUltraLight.ttf' },
  { family: 'NanumBarunGothic', weight: 300, file: 'NanumBarunGothicLight.ttf' },
  { family: 'NanumBarunGothic', weight: 400, file: 'NanumBarunGothic.ttf' },
  { family: 'NanumBarunGothic', weight: 700, file: 'NanumBarunGothicBold.ttf' },
  { family: 'RIDIBatang', weight: 400, file: 'RIDIBatang-Regular.ttf' },
  { family: 'KoPubWorldBatang', weight: 300, file: 'KoPubWorld Batang Light.ttf' },
  { family: 'KoPubWorldBatang', weight: 500, file: 'KoPubWorld Batang Medium.ttf' },
  { family: 'KoPubWorldBatang', weight: 700, file: 'KoPubWorld Batang Bold.ttf' },
  { family: 'NanumMyeongjo', weight: 400, file: 'NanumMyeongjo.ttf' },
  { family: 'NanumMyeongjo', weight: 700, file: 'NanumMyeongjoBold.ttf' },
  { family: 'NanumMyeongjo', weight: 800, file: 'NanumMyeongjoExtraBold.ttf' },
];

const FALLBACK_FONTS: Record<WritingSystem, FontInfo[]> = {
  latin: [{ family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.ttf' }],
  korean: [{ family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.ttf' }],
  japanese: [
    { family: 'Noto Sans JP', weight: 400, file: 'NotoSansJP-Regular.ttf' },
    { family: 'Noto Sans JP', weight: 700, file: 'NotoSansJP-Bold.ttf' },
  ],
  chinese: [
    { family: 'Noto Sans SC', weight: 400, file: 'NotoSansSC-Regular.ttf' },
    { family: 'Noto Sans SC', weight: 700, file: 'NotoSansSC-Bold.ttf' },
  ],
  emoji: [{ family: 'NotoColorEmoji', weight: 400, file: 'NotoColorEmoji.ttf' }],
};

export const FONT_CDN_BASE = 'https://cdn.typie.net/fonts/editor';
const FONT_CACHE_NAME = 'typie-fonts';

const loadedFonts = new WeakMap<Application, Set<string>>();
const loadingFonts = new WeakMap<Application, Map<string, Promise<void>>>();

function getLoadedSet(app: Application): Set<string> {
  let set = loadedFonts.get(app);
  if (!set) {
    set = new Set();
    loadedFonts.set(app, set);
  }
  return set;
}

function getLoadingMap(app: Application): Map<string, Promise<void>> {
  let map = loadingFonts.get(app);
  if (!map) {
    map = new Map();
    loadingFonts.set(app, map);
  }
  return map;
}

async function fetchFont(url: string): Promise<ArrayBuffer> {
  const cache = await caches.open(FONT_CACHE_NAME);
  const cached = await cache.match(url);
  if (cached) return cached.arrayBuffer();

  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to fetch font from ${url}`);

  await cache.put(url, response.clone());
  return response.arrayBuffer();
}

async function addFont(app: Application, font: FontInfo): Promise<void> {
  const key = `${font.family}-${font.weight}`;
  const loaded = getLoadedSet(app);
  if (loaded.has(key)) return;

  const loading = getLoadingMap(app);
  const existing = loading.get(key);
  if (existing) {
    await existing;
    return;
  }

  const promise = (async () => {
    try {
      const buffer = await fetchFont(`${FONT_CDN_BASE}/${font.file}`);
      if (loaded.has(key)) return;
      app.addFont(font.family, font.weight, new Uint8Array(buffer));
      loaded.add(key);
    } catch (err) {
      console.warn(`Failed to load font ${font.family} (${font.weight}):`, err);
    }
  })();

  loading.set(key, promise);
  try {
    await promise;
  } finally {
    loading.delete(key);
  }
}

export async function ensurePhantomFonts(app: Application): Promise<void> {
  await Promise.all(PHANTOM_FONTS.map((font) => addFont(app, font)));
  for (const font of PHANTOM_FONTS) {
    app.registerFallbackFont(font.family);
  }
}

export async function ensureRequiredFonts(app: Application, fonts: [string, number][]): Promise<boolean> {
  const loaded = getLoadedSet(app);
  const toLoad = fonts
    .filter(([family, weight]) => !loaded.has(`${family}-${weight}`))
    .map(([family, weight]) => DEFAULT_FONTS.find((f) => f.family === family && f.weight === weight))
    .filter((f): f is FontInfo => f !== undefined);

  if (toLoad.length === 0) return false;

  await Promise.all(toLoad.map((font) => addFont(app, font)));
  return true;
}

export async function ensureRequiredWritingSystems(app: Application, systems: WritingSystem[]): Promise<boolean> {
  const loaded = getLoadedSet(app);
  const toLoad = systems.flatMap((system) => FALLBACK_FONTS[system]).filter((font) => !loaded.has(`${font.family}-${font.weight}`));

  if (toLoad.length === 0) return false;

  await Promise.all(toLoad.map((font) => addFont(app, font)));

  const families = new Set(toLoad.map((f) => f.family));
  for (const family of families) {
    app.registerFallbackFont(family);
  }

  return true;
}

export function getAvailableFontsMap(): Record<string, number[]> {
  const map: Record<string, number[]> = {};
  for (const font of DEFAULT_FONTS) {
    if (!map[font.family]) {
      map[font.family] = [];
    }
    map[font.family].push(font.weight);
  }
  return map;
}
