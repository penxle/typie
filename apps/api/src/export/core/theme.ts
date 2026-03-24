import themeData from '@typie/editor/theme.json' with { type: 'json' };
import type { Theme } from '@typie/editor';

const colorToU32 = (color: string): number => {
  const clean = color.replace('#', '');
  if (clean.length !== 6) {
    return 0x00_00_00_ff;
  }
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return ((r << 24) | (g << 16) | (b << 8) | 0xff) >>> 0;
};

const LIGHT_COLORS: Record<string, string> = {
  ...themeData.shared,
  ...themeData.lightShared,
  ...themeData.variants['light-white'],
};

const DARK_COLORS: Record<string, string> = {
  ...themeData.shared,
  ...themeData.darkShared,
  ...themeData.variants['dark-black'],
};

/** 테마 색상 키를 hex 문자열(# 없이)로 변환. 매칭 실패 시 undefined 반환. */
export const resolveColorToHex = (colorKey: string): string | undefined => {
  const hex = LIGHT_COLORS[colorKey];
  return hex ? hex.replace('#', '') : undefined;
};

const buildTheme = (colors: Record<string, string>): Theme => ({
  colors: new Map(Object.entries(colors).map(([key, value]) => [key, colorToU32(value)])),
});

export const LIGHT_THEME: Theme = buildTheme(LIGHT_COLORS);
export const DARK_THEME: Theme = buildTheme(DARK_COLORS);
