import type { Application } from '@penxle/editor';
import type { WritingSystem } from './types';

export type FontInfo = {
  name: string;
  weight: number;
  file: string;
};

export const EDITOR_FONTS: FontInfo[] = [
  { name: 'Pretendard', weight: 100, file: 'Pretendard-Thin.ttf' },
  { name: 'Pretendard', weight: 200, file: 'Pretendard-ExtraLight.ttf' },
  { name: 'Pretendard', weight: 300, file: 'Pretendard-Light.ttf' },
  { name: 'Pretendard', weight: 400, file: 'Pretendard-Regular.ttf' },
  { name: 'Pretendard', weight: 500, file: 'Pretendard-Medium.ttf' },
  { name: 'Pretendard', weight: 600, file: 'Pretendard-SemiBold.ttf' },
  { name: 'Pretendard', weight: 700, file: 'Pretendard-Bold.ttf' },
  { name: 'Pretendard', weight: 800, file: 'Pretendard-ExtraBold.ttf' },
  { name: 'Pretendard', weight: 900, file: 'Pretendard-Black.ttf' },
  { name: 'KoPubWorldDotum', weight: 300, file: 'KoPubWorld Dotum Light.ttf' },
  { name: 'KoPubWorldDotum', weight: 500, file: 'KoPubWorld Dotum Medium.ttf' },
  { name: 'KoPubWorldDotum', weight: 700, file: 'KoPubWorld Dotum Bold.ttf' },
  { name: 'NanumBarunGothic', weight: 200, file: 'NanumBarunGothicUltraLight.ttf' },
  { name: 'NanumBarunGothic', weight: 300, file: 'NanumBarunGothicLight.ttf' },
  { name: 'NanumBarunGothic', weight: 400, file: 'NanumBarunGothic.ttf' },
  { name: 'NanumBarunGothic', weight: 700, file: 'NanumBarunGothicBold.ttf' },
  { name: 'RIDIBatang', weight: 400, file: 'RIDIBatang-Regular.ttf' },
  { name: 'KoPubWorldBatang', weight: 300, file: 'KoPubWorld Batang Light.ttf' },
  { name: 'KoPubWorldBatang', weight: 500, file: 'KoPubWorld Batang Medium.ttf' },
  { name: 'KoPubWorldBatang', weight: 700, file: 'KoPubWorld Batang Bold.ttf' },
  { name: 'NanumMyeongjo', weight: 400, file: 'NanumMyeongjo.ttf' },
  { name: 'NanumMyeongjo', weight: 700, file: 'NanumMyeongjoBold.ttf' },
  { name: 'NanumMyeongjo', weight: 800, file: 'NanumMyeongjoExtraBold.ttf' },
];

export const FONT_CDN_BASE = 'https://cdn.typie.net/fonts/editor';
export const EMOJI_FONT_URL = 'https://cdn.typie.net/fonts/editor/NotoColorEmoji.ttf';

const loadedFonts = new Set<string>();
const loadedSystems = new Set<WritingSystem>();
const loadingPromises = new Map<string, Promise<void>>();

type FontConfig = {
  family: string;
  weight: number;
  url: string;
};

const WRITING_SYSTEM_FONT_MAP: Record<WritingSystem, FontConfig[]> = {
  latin: [],
  korean: [],
  japanese: [
    { family: 'Noto Sans JP', weight: 400, url: 'https://cdn.typie.net/fonts/fallback/NotoSansJP-Regular.ttf' },
    { family: 'Noto Sans JP', weight: 700, url: 'https://cdn.typie.net/fonts/fallback/NotoSansJP-Bold.ttf' },
  ],
  chinese: [
    { family: 'Noto Sans SC', weight: 400, url: 'https://cdn.typie.net/fonts/fallback/NotoSansSC-Regular.ttf' },
    { family: 'Noto Sans SC', weight: 700, url: 'https://cdn.typie.net/fonts/fallback/NotoSansSC-Bold.ttf' },
  ],
};

export async function loadFont(app: Application, name: string, weight: number): Promise<void> {
  const key = `${name}-${weight}`;
  if (loadedFonts.has(key)) return;
  if (loadingPromises.has(key)) return loadingPromises.get(key);

  const fontInfo = EDITOR_FONTS.find((f) => f.name === name && f.weight === weight);
  if (!fontInfo) return;

  const promise = (async () => {
    const response = await fetch(`${FONT_CDN_BASE}/${fontInfo.file}`);
    const buffer = await response.arrayBuffer();
    app.registerFont(name, weight, new Uint8Array(buffer));
    loadedFonts.add(key);
  })();

  loadingPromises.set(key, promise);

  try {
    await promise;
  } finally {
    loadingPromises.delete(key);
  }
}

export async function loadInitialFonts(app: Application): Promise<void> {
  await loadFont(app, 'Pretendard', 400);
}

export async function loadEmojiFallback(app: Application): Promise<void> {
  const key = 'NotoColorEmoji-400';
  if (loadedFonts.has(key)) return;

  const response = await fetch(EMOJI_FONT_URL);
  const buffer = await response.arrayBuffer();
  app.registerFallbackFont('NotoColorEmoji', 400, new Uint8Array(buffer));
  loadedFonts.add(key);
}

export async function ensureRequiredFonts(app: Application, fonts: [string, number][]): Promise<boolean> {
  const fontsToLoad = fonts.filter(([name, weight]) => !loadedFonts.has(`${name}-${weight}`));
  if (fontsToLoad.length === 0) return false;

  await Promise.all(fontsToLoad.map(([name, weight]) => loadFont(app, name, weight)));
  return true;
}

export async function ensureRequiredScripts(app: Application, systems: WritingSystem[]): Promise<boolean> {
  const systemsToLoad = systems.filter((s) => !loadedSystems.has(s) && WRITING_SYSTEM_FONT_MAP[s].length > 0);
  if (systemsToLoad.length === 0) return false;

  for (const system of systemsToLoad) {
    const fonts = WRITING_SYSTEM_FONT_MAP[system];
    for (const font of fonts) {
      try {
        const response = await fetch(font.url);
        if (!response.ok) continue;
        const buffer = await response.arrayBuffer();
        app.registerFallbackFont(font.family, font.weight, new Uint8Array(buffer));
      } catch (err) {
        console.warn(`Failed to load font ${font.family}:`, err);
      }
    }
    loadedSystems.add(system);
  }

  return true;
}

export function isFontLoaded(name: string, weight: number): boolean {
  return loadedFonts.has(`${name}-${weight}`);
}

export function isSystemLoaded(system: WritingSystem): boolean {
  return loadedSystems.has(system);
}

export function getAvailableFontsMap(): Record<string, number[]> {
  const map: Record<string, number[]> = {};
  for (const font of EDITOR_FONTS) {
    if (!map[font.name]) {
      map[font.name] = [];
    }
    map[font.name].push(font.weight);
  }
  return map;
}
