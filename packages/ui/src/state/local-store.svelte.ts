import { browser } from '$app/environment';

export class LocalStore<T> {
  #key: string;
  current = $state<T>() as T;

  constructor(key: string, defaultValue: T) {
    this.#key = key;
    this.current = defaultValue;

    if (browser) {
      const item = localStorage.getItem(this.#key);
      if (item) {
        const value = JSON.parse(item);
        this.current = { ...defaultValue, ...value };
      }
    }

    $effect(() => {
      localStorage.setItem(this.#key, JSON.stringify(this.current));
    });
  }
}
