import themeData from '@typie/editor/theme.json' with { type: 'json' };
import type { DarkVariant, EffectiveTheme, LightVariant } from '@typie/ui/context';

export type ThemeColors = {
  colors: Map<string, number>;
};

export type ThemeVariant = `light-${LightVariant}` | `dark-${DarkVariant}`;

const colorToU32 = (color: string, alpha = 0xff): number => {
  const clean = color.replace('#', '').replace(/^var\(.+\)$/, '');
  if (clean.length < 6) {
    console.warn(`Invalid color format: ${color}, falling back to black`);
    return 0x00_00_00_ff;
  }
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return ((r << 24) | (g << 16) | (b << 8) | alpha) >>> 0;
};

const assembleVariant = (variant: string): Record<string, string> => {
  const isLight = variant.startsWith('light-');
  return {
    ...themeData.shared,
    ...(isLight ? themeData.lightShared : themeData.darkShared),
    ...themeData.variants[variant as keyof typeof themeData.variants],
  };
};

export const THEME_COLORS: Record<ThemeVariant, Record<string, string>> = Object.fromEntries(
  Object.keys(themeData.variants).map((variant) => [variant, assembleVariant(variant)]),
) as Record<ThemeVariant, Record<string, string>>;

const buildTheme = (rawColors: Record<string, string>): ThemeColors => ({
  colors: new Map(Object.entries(rawColors).map(([k, v]) => [k, colorToU32(v)] as [string, number])),
});

export const getEditorTheme = (effectiveTheme: EffectiveTheme, lightVariant: LightVariant, darkVariant: DarkVariant): ThemeColors => {
  const variant: ThemeVariant = effectiveTheme === 'light' ? `light-${lightVariant}` : `dark-${darkVariant}`;
  return buildTheme(THEME_COLORS[variant]);
};
