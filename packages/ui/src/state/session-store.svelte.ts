import { browser } from '$app/environment';
import { safeJsonParse } from '../utils';

export class SessionStore<T> {
  #key: string;
  current = $state<T>() as T;

  constructor(key: string, value: T) {
    this.#key = key;
    this.current = value;

    if (browser) {
      const item = sessionStorage.getItem(this.#key);
      if (item) {
        this.current = safeJsonParse<T>(item, value);
      }
    }

    $effect(() => {
      if (this.current !== undefined) {
        sessionStorage.setItem(this.#key, JSON.stringify(this.current));
      }
    });
  }
}
