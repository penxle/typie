import { DARK_VARIANTS, DEFAULT_DARK_VARIANT, DEFAULT_LIGHT_VARIANT, LIGHT_VARIANTS, VARIANT_CANVAS } from '@typie/styled-system/presets';
import { nativeTheme, session } from 'electron';
import type { ThemeVariant } from '@typie/styled-system/presets';

export type Theme = 'light' | 'dark';
export type ThemePayload = { theme: Theme; variantLight: string; variantDark: string };

const COOKIE_THEME = 'typie-th';
const COOKIE_LIGHT_VARIANT = 'typie-th-lv';
const COOKIE_DARK_VARIANT = 'typie-th-dv';

const LIGHT = new Set<string>(LIGHT_VARIANTS);
const DARK = new Set<string>(DARK_VARIANTS);

export const osTheme = (): Theme => (nativeTheme.shouldUseDarkColors ? 'dark' : 'light');

export const defaultThemePayload = (theme: Theme): ThemePayload => ({
  theme,
  variantLight: DEFAULT_LIGHT_VARIANT,
  variantDark: DEFAULT_DARK_VARIANT,
});

const canvasOf = ({ theme, variantLight, variantDark }: ThemePayload): string => {
  const key = `${theme}-${theme === 'dark' ? variantDark : variantLight}`;
  const fallback = theme === 'dark' ? `dark-${DEFAULT_DARK_VARIANT}` : `light-${DEFAULT_LIGHT_VARIANT}`;
  return VARIANT_CANVAS[key as ThemeVariant] ?? VARIANT_CANVAS[fallback as ThemeVariant];
};

export const themeColors = (payload: ThemePayload) => ({
  background: canvasOf(payload),
  symbol: payload.theme === 'dark' ? '#c8c8c8' : '#888888',
});

export const readStoredTheme = async (websiteUrl: string): Promise<ThemePayload> => {
  const cookies = await session.defaultSession.cookies.get({ url: websiteUrl }).catch(() => []);
  const get = (name: string) => cookies.find((cookie) => cookie.name === name)?.value;
  const preference = get(COOKIE_THEME);
  const light = get(COOKIE_LIGHT_VARIANT);
  const dark = get(COOKIE_DARK_VARIANT);
  return {
    theme: preference === 'light' || preference === 'dark' ? preference : osTheme(),
    variantLight: light && LIGHT.has(light) ? light : DEFAULT_LIGHT_VARIANT,
    variantDark: dark && DARK.has(dark) ? dark : DEFAULT_DARK_VARIANT,
  };
};
