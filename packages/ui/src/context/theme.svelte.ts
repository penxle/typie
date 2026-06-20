import { getContext, setContext } from 'svelte';
import { MediaQuery } from 'svelte/reactivity';
import Cookies from 'universal-cookie';
import { browser } from '$app/environment';
import { page } from '$app/state';
import type { CookieChangeOptions } from 'universal-cookie';

export type Theme = 'light' | 'dark' | 'auto';
export type EffectiveTheme = Exclude<Theme, 'auto'>;

export type LightVariant = 'white' | 'snow' | 'butter' | 'peach' | 'rose' | 'lavender' | 'mint' | 'latte';
export type DarkVariant = 'black' | 'charcoal' | 'graphite' | 'midnight' | 'navy' | 'obsidian' | 'storm' | 'espresso';
export type ThemeVariant = `light-${LightVariant}` | `dark-${DarkVariant}`;

const LIGHT_VARIANTS = new Set<LightVariant>(['white', 'snow', 'butter', 'peach', 'rose', 'lavender', 'mint', 'latte']);
const DARK_VARIANTS = new Set<DarkVariant>(['black', 'charcoal', 'graphite', 'midnight', 'navy', 'obsidian', 'storm', 'espresso']);

const COOKIE = 'typie-th';
const COOKIE_LIGHT_VARIANT = 'typie-th-lv';
const COOKIE_DARK_VARIANT = 'typie-th-dv';

export class ThemeState {
  #cookies = new Cookies();

  #overrideTheme = $state<EffectiveTheme>();
  #currentTheme = $state<Theme>('auto');
  #effectiveTheme = $derived.by<EffectiveTheme>(() => {
    const value = this.#overrideTheme ?? this.#currentTheme;
    if (value === 'auto') {
      return this.#prefersDark.current ? 'dark' : 'light';
    }
    return value;
  });

  #lightVariant = $state<LightVariant>('white');
  #darkVariant = $state<DarkVariant>('black');

  #prefersDark = new MediaQuery('(prefers-color-scheme: dark)');

  constructor() {
    const defaultTheme = page.url.pathname.includes('_webview') ? 'light' : 'auto';

    const value = this.#cookies.get(COOKIE);
    this.#currentTheme = value && ['auto', 'light', 'dark'].includes(value) ? value : defaultTheme;

    const lightVariantValue = this.#cookies.get(COOKIE_LIGHT_VARIANT);
    this.#lightVariant = lightVariantValue && LIGHT_VARIANTS.has(lightVariantValue) ? lightVariantValue : 'white';

    const darkVariantValue = this.#cookies.get(COOKIE_DARK_VARIANT);
    this.#darkVariant = darkVariantValue && DARK_VARIANTS.has(darkVariantValue) ? darkVariantValue : 'black';

    if (browser) {
      document.documentElement.dataset.theme = this.#effectiveTheme;
      document.documentElement.dataset.variantLight = this.#lightVariant;
      document.documentElement.dataset.variantDark = this.#darkVariant;
    }

    $effect(() => {
      void this.#effectiveTheme;
      void this.#lightVariant;
      void this.#darkVariant;

      if (
        document.documentElement.dataset.theme !== this.#effectiveTheme ||
        document.documentElement.dataset.variantLight !== this.#lightVariant ||
        document.documentElement.dataset.variantDark !== this.#darkVariant
      ) {
        document.documentElement.dataset.noTransition = '';
        if (document.startViewTransition) {
          document
            .startViewTransition(() => {
              document.documentElement.dataset.theme = this.#effectiveTheme;
              document.documentElement.dataset.variantLight = this.#lightVariant;
              document.documentElement.dataset.variantDark = this.#darkVariant;
            })
            .finished.then(() => {
              delete document.documentElement.dataset.noTransition;
            });
        } else {
          document.documentElement.dataset.theme = this.#effectiveTheme;
          document.documentElement.dataset.variantLight = this.#lightVariant;
          document.documentElement.dataset.variantDark = this.#darkVariant;
          setTimeout(() => {
            delete document.documentElement.dataset.noTransition;
          }, 0);
        }
      }
    });

    $effect(() => {
      const handler = ({ name, value }: CookieChangeOptions) => {
        if (name === COOKIE) {
          this.#currentTheme = value && ['auto', 'light', 'dark'].includes(value) ? value : defaultTheme;
        } else if (name === COOKIE_LIGHT_VARIANT) {
          this.#lightVariant = value && LIGHT_VARIANTS.has(value) ? value : 'white';
        } else if (name === COOKIE_DARK_VARIANT) {
          this.#darkVariant = value && DARK_VARIANTS.has(value) ? value : 'black';
        }
      };

      this.#cookies.addChangeListener(handler);

      return () => {
        this.#cookies.removeChangeListener(handler);
      };
    });
  }

  get currentTheme(): Theme {
    return this.#currentTheme;
  }

  set currentTheme(theme: Theme) {
    this.#cookies.set(COOKIE, theme, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
  }

  get effectiveTheme(): EffectiveTheme {
    return this.#effectiveTheme;
  }

  get currentThemeVariant(): ThemeVariant {
    return this.#effectiveTheme === 'light' ? `light-${this.#lightVariant}` : `dark-${this.#darkVariant}`;
  }

  get lightVariant(): LightVariant {
    return this.#lightVariant;
  }

  set lightVariant(variant: LightVariant) {
    this.#cookies.set(COOKIE_LIGHT_VARIANT, variant, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
  }

  get darkVariant(): DarkVariant {
    return this.#darkVariant;
  }

  set darkVariant(variant: DarkVariant) {
    this.#cookies.set(COOKIE_DARK_VARIANT, variant, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
  }

  get overrideTheme(): EffectiveTheme | undefined {
    return this.#overrideTheme;
  }

  set overrideTheme(theme: EffectiveTheme | undefined) {
    this.#overrideTheme = theme;
  }
}

const key: unique symbol = Symbol('ThemeContext');

export const getThemeContext = () => {
  return getContext<ThemeState>(key);
};

export const setupThemeContext = () => {
  const themeState = $state<ThemeState>(new ThemeState());

  setContext(key, themeState);

  return themeState;
};
