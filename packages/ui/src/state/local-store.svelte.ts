import { browser } from '$app/environment';
import { safeJsonParse } from '../utils';

export class LocalStore<T> {
  #key: string;
  current = $state<T>() as T;

  constructor(key: string, defaultValue: T) {
    this.#key = key;
    this.current = defaultValue;

    if (browser) {
      const item = localStorage.getItem(this.#key);
      if (item) {
        const value = safeJsonParse<T>(item, defaultValue);
        this.current = { ...defaultValue, ...value };
      }
    }

    $effect(() => {
      if (this.current !== undefined) {
        localStorage.setItem(this.#key, JSON.stringify(this.current));
      }
    });
  }
}
