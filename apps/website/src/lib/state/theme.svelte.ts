import { MediaQuery } from 'svelte/reactivity';
import Cookies from 'universal-cookie';
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

  get current(): Theme {
    return this.#current;
  }

  set current(theme: Theme) {
    this.#current = theme;

    this.#cookies.set(COOKIE, theme, { path: '/', maxAge: 365 * 24 * 60 * 60, sameSite: 'lax' });
    document.documentElement.dataset.theme = theme;
  }

  get effective(): EffectiveTheme {
    return this.#effective;
  }

  constructor() {
    $effect(() => {
      const value = this.#cookies.get(COOKIE);
      this.#current = value && ['light', 'dark'].includes(value) ? value : 'auto';
      document.documentElement.dataset.theme = this.#current;

      const handler = ({ name, value }: CookieChangeOptions) => {
        if (name === COOKIE) {
          this.#current = value && ['light', 'dark'].includes(value) ? value : 'auto';
          document.documentElement.dataset.theme = this.#current;
        }
      };

      this.#cookies.addChangeListener(handler);

      return () => {
        this.#cookies.removeChangeListener(handler);
      };
    });
  }
}
