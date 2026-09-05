import { DARK_VARIANTS, DEFAULT_DARK_VARIANT, DEFAULT_LIGHT_VARIANT, LIGHT_VARIANTS } from '@typie/styled-system/presets';

export type ThemeAttributes = { theme: string; variantLight: string; variantDark: string };

const DARK_BLACK: ThemeAttributes = { theme: 'dark', variantLight: DEFAULT_LIGHT_VARIANT, variantDark: DEFAULT_DARK_VARIANT };

export const FORCED_ROUTES: { prefix: string; attributes: ThemeAttributes }[] = [
  { prefix: '/website/(landing)', attributes: DARK_BLACK },
  { prefix: '/website/admin', attributes: DARK_BLACK },
  { prefix: '/website/legal', attributes: DARK_BLACK },
];

const THEMES = new Set(['auto', 'light', 'dark']);
const LIGHT = new Set<string>(LIGHT_VARIANTS);
const DARK = new Set<string>(DARK_VARIANTS);

export const resolveThemeAttributes = (input: {
  routeId: string | null;
  pathname: string;
  cookies: { theme?: string; light?: string; dark?: string };
}): ThemeAttributes => {
  const routeId = input.routeId;
  const forced = routeId ? FORCED_ROUTES.find(({ prefix }) => routeId.startsWith(prefix)) : undefined;
  if (forced) return forced.attributes;
  const defaultTheme = input.pathname.includes('_webview') ? 'light' : 'auto';
  const theme = input.cookies.theme && THEMES.has(input.cookies.theme) ? input.cookies.theme : defaultTheme;
  const variantLight = input.cookies.light && LIGHT.has(input.cookies.light) ? input.cookies.light : DEFAULT_LIGHT_VARIANT;
  const variantDark = input.cookies.dark && DARK.has(input.cookies.dark) ? input.cookies.dark : DEFAULT_DARK_VARIANT;
  return { theme, variantLight, variantDark };
};
