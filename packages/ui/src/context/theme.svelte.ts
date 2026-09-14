import { DARK_VARIANTS, DEFAULT_DARK_VARIANT, DEFAULT_LIGHT_VARIANT, LIGHT_VARIANTS } from '@typie/styled-system/presets';
import { getAbortSignal } from 'svelte';
import { on } from 'svelte/events';
import { MediaQuery } from 'svelte/reactivity';
import Cookies from 'universal-cookie';
import { browser } from '$app/environment';
import { page } from '$app/state';
import { createStableContext } from './stable-context';
import type { DarkVariant, LightVariant, ThemeVariant } from '@typie/styled-system/presets';
import type { CookieChangeOptions } from 'universal-cookie';

export type { DarkVariant, LightVariant, ThemeVariant } from '@typie/styled-system/presets';

export type Theme = 'light' | 'dark' | 'auto';
export type EffectiveTheme = Exclude<Theme, 'auto'>;

const LIGHT = new Set<string>(LIGHT_VARIANTS);
const DARK = new Set<string>(DARK_VARIANTS);

const isLightVariant = (value: unknown): value is LightVariant => typeof value === 'string' && LIGHT.has(value);
const isDarkVariant = (value: unknown): value is DarkVariant => typeof value === 'string' && DARK.has(value);
const cookieLightVariant = (value: unknown): LightVariant => (isLightVariant(value) ? value : DEFAULT_LIGHT_VARIANT);
const cookieDarkVariant = (value: unknown): DarkVariant => (isDarkVariant(value) ? value : DEFAULT_DARK_VARIANT);

const COOKIE = 'typie-th';
const COOKIE_LIGHT_VARIANT = 'typie-th-lv';
const COOKIE_DARK_VARIANT = 'typie-th-dv';

const browserForcesDarkTheme = (): boolean => {
  if (window.matchMedia('(forced-colors: active)').matches) return false;

  // Auto Dark Theme can override colors without reporting a dark media query.
  // Request light on the probe independently of the current page theme.
  // https://developer.chrome.com/blog/auto-dark-theme
  const probe = document.createElement('div');
  probe.style.cssText = 'display: none; color-scheme: light; background-color: Canvas';
  document.documentElement.append(probe);
  const color = window.getComputedStyle(probe).backgroundColor;
  probe.remove();
  return color.startsWith('rgb(') && color !== 'rgb(255, 255, 255)';
};

export class ThemeState {
  #cookies = new Cookies();

  #overrideTheme = $state<EffectiveTheme>();
  #currentTheme = $state<Theme>('auto');
  #browserForcesDark = $state(false);
  #effectiveTheme = $derived.by<EffectiveTheme>(() => {
    if (this.#browserForcesDark) return 'dark';
    const value = this.#overrideTheme ?? this.#currentTheme;
    if (value === 'auto') {
      return this.#prefersDark.current ? 'dark' : 'light';
    }
    return value;
  });

  #lightVariant = $state<LightVariant>(DEFAULT_LIGHT_VARIANT);
  #darkVariant = $state<DarkVariant>(DEFAULT_DARK_VARIANT);

  #overrideVariant = $state<{ light?: LightVariant; dark?: DarkVariant }>();
  #effectiveLightVariant = $derived.by<LightVariant>(() => this.#overrideVariant?.light ?? this.#lightVariant);
  #effectiveDarkVariant = $derived.by<DarkVariant>(() => this.#overrideVariant?.dark ?? this.#darkVariant);

  #prefersDark = new MediaQuery('(prefers-color-scheme: dark)');

  constructor() {
    const defaultTheme = page.url.pathname.includes('_webview') ? 'light' : 'auto';

    const value = this.#cookies.get(COOKIE);
    this.#currentTheme = value && ['auto', 'light', 'dark'].includes(value) ? value : defaultTheme;

    this.#lightVariant = cookieLightVariant(this.#cookies.get(COOKIE_LIGHT_VARIANT));
    this.#darkVariant = cookieDarkVariant(this.#cookies.get(COOKIE_DARK_VARIANT));

    if (browser) {
      const stamped = document.documentElement.dataset;
      const stampedTheme = stamped.theme === 'light' || stamped.theme === 'dark' ? stamped.theme : undefined;
      if (stampedTheme && stampedTheme !== this.#currentTheme) {
        this.#overrideTheme = stampedTheme;
      }
      const override: { light?: LightVariant; dark?: DarkVariant } = {};
      if (isLightVariant(stamped.variantLight) && stamped.variantLight !== this.#lightVariant) {
        override.light = stamped.variantLight;
      }
      if (isDarkVariant(stamped.variantDark) && stamped.variantDark !== this.#darkVariant) {
        override.dark = stamped.variantDark;
      }
      if (override.light || override.dark) {
        this.#overrideVariant = override;
      }
    }

    if (browser) {
      this.#browserForcesDark = browserForcesDarkTheme();
      document.documentElement.dataset.theme = this.#effectiveTheme;
      document.documentElement.dataset.variantLight = this.#effectiveLightVariant;
      document.documentElement.dataset.variantDark = this.#effectiveDarkVariant;
    }

    $effect(() => {
      void this.#prefersDark.current;
      const refresh = () => {
        this.#browserForcesDark = browserForcesDarkTheme();
      };
      refresh();
      const options = { signal: getAbortSignal() };
      on(window, 'focus', refresh, options);
      on(window, 'pageshow', refresh, options);
      on(document, 'visibilitychange', refresh, options);
    });

    $effect(() => {
      void this.#effectiveTheme;
      void this.#effectiveLightVariant;
      void this.#effectiveDarkVariant;

      if (
        document.documentElement.dataset.theme !== this.#effectiveTheme ||
        document.documentElement.dataset.variantLight !== this.#effectiveLightVariant ||
        document.documentElement.dataset.variantDark !== this.#effectiveDarkVariant
      ) {
        document.documentElement.dataset.noTransition = '';
        if (document.startViewTransition) {
          document
            .startViewTransition(() => {
              document.documentElement.dataset.theme = this.#effectiveTheme;
              document.documentElement.dataset.variantLight = this.#effectiveLightVariant;
              document.documentElement.dataset.variantDark = this.#effectiveDarkVariant;
            })
            .finished.then(() => {
              delete document.documentElement.dataset.noTransition;
            });
        } else {
          document.documentElement.dataset.theme = this.#effectiveTheme;
          document.documentElement.dataset.variantLight = this.#effectiveLightVariant;
          document.documentElement.dataset.variantDark = this.#effectiveDarkVariant;
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
          this.#lightVariant = cookieLightVariant(value);
        } else if (name === COOKIE_DARK_VARIANT) {
          this.#darkVariant = cookieDarkVariant(value);
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
    return this.#effectiveTheme === 'light' ? `light-${this.#effectiveLightVariant}` : `dark-${this.#effectiveDarkVariant}`;
  }

  get lightVariant(): LightVariant {
    return this.#effectiveLightVariant;
  }

  set lightVariant(variant: LightVariant) {
    this.#cookies.set(COOKIE_LIGHT_VARIANT, variant, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
  }

  get darkVariant(): DarkVariant {
    return this.#effectiveDarkVariant;
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

  get overrideVariant(): { light?: LightVariant; dark?: DarkVariant } | undefined {
    return this.#overrideVariant;
  }

  set overrideVariant(variant: { light?: LightVariant; dark?: DarkVariant } | undefined) {
    this.#overrideVariant = variant;
  }
}

const [getThemeContext, setThemeContext] = createStableContext<ThemeState>('ui.ThemeContext');

export { getThemeContext };

export const setupThemeContext = () => {
  const themeState = $state<ThemeState>(new ThemeState());

  setThemeContext(themeState);

  return themeState;
};
