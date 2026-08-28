import { nativeTheme, session } from 'electron';

export type Theme = 'light' | 'dark';
export type ThemePayload = { theme: Theme; variantLight: string; variantDark: string };

const COOKIE_THEME = 'typie-th';
const COOKIE_LIGHT_VARIANT = 'typie-th-lv';
const COOKIE_DARK_VARIANT = 'typie-th-dv';

export const osTheme = (): Theme => (nativeTheme.shouldUseDarkColors ? 'dark' : 'light');

export const themeColors = (theme: Theme) =>
  theme === 'dark' ? { background: '#1a1a1a', symbol: '#c8c8c8' } : { background: '#ffffff', symbol: '#888888' };

export const readStoredTheme = async (websiteUrl: string): Promise<ThemePayload> => {
  const cookies = await session.defaultSession.cookies.get({ url: websiteUrl }).catch(() => []);
  const get = (name: string) => cookies.find((cookie) => cookie.name === name)?.value;
  const preference = get(COOKIE_THEME);
  return {
    theme: preference === 'light' || preference === 'dark' ? preference : osTheme(),
    variantLight: get(COOKIE_LIGHT_VARIANT) ?? 'white',
    variantDark: get(COOKIE_DARK_VARIANT) ?? 'black',
  };
};
