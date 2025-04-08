import { browser } from '$app/environment';

export class SessionStore<T> {
  #key: string;
  current = $state<T>() as T;

  constructor(key: string, value: T) {
    this.#key = key;
    this.current = value;

    if (browser) {
      const item = sessionStorage.getItem(this.#key);
      if (item) {
        this.current = JSON.parse(item);
      }
    }

    $effect(() => {
      sessionStorage.setItem(this.#key, JSON.stringify(this.current));
    });
  }
}
