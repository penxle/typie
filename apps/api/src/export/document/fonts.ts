import type { Application } from '@typie/editor';

const FONT_CDN_BASE = 'https://cdn.typie.net/fonts/editor';

const FONT_FILES: Record<string, string> = {
  'Pretendard-100': 'Pretendard-Thin.ttf',
  'Pretendard-200': 'Pretendard-ExtraLight.ttf',
  'Pretendard-300': 'Pretendard-Light.ttf',
  'Pretendard-400': 'Pretendard-Regular.ttf',
  'Pretendard-500': 'Pretendard-Medium.ttf',
  'Pretendard-600': 'Pretendard-SemiBold.ttf',
  'Pretendard-700': 'Pretendard-Bold.ttf',
  'Pretendard-800': 'Pretendard-ExtraBold.ttf',
  'Pretendard-900': 'Pretendard-Black.ttf',
  'KoPubWorldDotum-300': 'KoPubWorld Dotum Light.ttf',
  'KoPubWorldDotum-500': 'KoPubWorld Dotum Medium.ttf',
  'KoPubWorldDotum-700': 'KoPubWorld Dotum Bold.ttf',
  'NanumBarunGothic-200': 'NanumBarunGothicUltraLight.ttf',
  'NanumBarunGothic-300': 'NanumBarunGothicLight.ttf',
  'NanumBarunGothic-400': 'NanumBarunGothic.ttf',
  'NanumBarunGothic-700': 'NanumBarunGothicBold.ttf',
  'RIDIBatang-400': 'RIDIBatang-Regular.ttf',
  'KoPubWorldBatang-300': 'KoPubWorld Batang Light.ttf',
  'KoPubWorldBatang-500': 'KoPubWorld Batang Medium.ttf',
  'KoPubWorldBatang-700': 'KoPubWorld Batang Bold.ttf',
  'NanumMyeongjo-400': 'NanumMyeongjo.ttf',
  'NanumMyeongjo-700': 'NanumMyeongjoBold.ttf',
  'NanumMyeongjo-800': 'NanumMyeongjoExtraBold.ttf',
};

const fontDataCache = new Map<string, Uint8Array>();

async function fetchFontData(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch font from ${url}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

async function loadFont(app: Application, name: string, weight: number): Promise<void> {
  const key = `${name}-${weight}`;
  const file = FONT_FILES[key];
  if (!file) return;

  let data = fontDataCache.get(key);
  if (!data) {
    data = await fetchFontData(`${FONT_CDN_BASE}/${file}`);
    fontDataCache.set(key, data);
  }

  app.registerFont(name, weight, data);
}

export async function loadInitialFonts(app: Application): Promise<void> {
  await loadFont(app, 'Pretendard', 400);
}

export async function ensureRequiredFonts(app: Application, fonts: [string, number][]): Promise<void> {
  await Promise.all(fonts.map(([name, weight]) => loadFont(app, name, weight)));
}

let availableFontsCache: Record<string, number[]> | null = null;

export function getAvailableFontsMap(): Record<string, number[]> {
  if (availableFontsCache) return availableFontsCache;

  const map: Record<string, number[]> = {};
  for (const key of Object.keys(FONT_FILES)) {
    const [name, weightStr] = key.split('-');
    const weight = Number(weightStr);
    (map[name] ??= []).push(weight);
  }

  availableFontsCache = map;
  return map;
}
