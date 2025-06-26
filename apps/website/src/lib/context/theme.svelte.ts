import { getContext, setContext } from 'svelte';
import { MediaQuery } from 'svelte/reactivity';
import Cookies from 'universal-cookie';
import { browser } from '$app/environment';
import type { CookieChangeOptions } from 'universal-cookie';

export type Theme = 'light' | 'dark' | 'auto';
export type EffectiveTheme = Exclude<Theme, 'auto'>;

const COOKIE = 'typie-th';

export class ThemeState {
  #cookies = new Cookies();

  #current = $state<Theme>('auto');
  #effective = $derived.by<EffectiveTheme>(() => {
    if (this.#current !== 'auto') return this.#current;
    return this.#prefersDark.current ? 'dark' : 'light';
  });

  #prefersDark = new MediaQuery('(prefers-color-scheme: dark)');

  constructor() {
    const value = this.#cookies.get(COOKIE);
    this.#current = value && ['light', 'dark'].includes(value) ? value : 'auto';

    if (browser) {
      document.documentElement.dataset.theme = this.#effective;
    }

    $effect(() => {
      if (document.documentElement.dataset.theme !== this.#effective) {
        document.documentElement.dataset.noTransition = '';
        if (document.startViewTransition) {
          document
            .startViewTransition(() => {
              document.documentElement.dataset.theme = this.#effective;
            })
            .finished.then(() => {
              delete document.documentElement.dataset.noTransition;
            });
        } else {
          document.documentElement.dataset.theme = this.#effective;
          setTimeout(() => {
            delete document.documentElement.dataset.noTransition;
          }, 0);
        }
      }
    });

    $effect(() => {
      const handler = ({ name, value }: CookieChangeOptions) => {
        if (name === COOKIE) {
          this.#current = value && ['light', 'dark'].includes(value) ? value : 'auto';
        }
      };

      this.#cookies.addChangeListener(handler);

      return () => {
        this.#cookies.removeChangeListener(handler);
      };
    });
  }

  get current(): Theme {
    return this.#current;
  }

  set current(theme: Theme) {
    this.#cookies.set(COOKIE, theme, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
  }

  get effective(): EffectiveTheme {
    return this.#effective;
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
