import { browser } from '$app/environment';

export class LocalStore<T> {
  #key: string;
  current = $state<T>() as T;

  constructor(key: string, value: T) {
    this.#key = key;
    this.current = value;

    if (browser) {
      const item = localStorage.getItem(this.#key);
      if (item) {
        this.current = JSON.parse(item);
      }
    }

    $effect(() => {
      localStorage.setItem(this.#key, JSON.stringify(this.current));
    });
  }

  static get<U>(key: string): U | null {
    if (!browser) {
      return null;
    }

    const item = localStorage.getItem(key);
    if (!item) {
      return null;
    }

    return JSON.parse(item) as U;
  }

  static set<U>(key: string, value: U) {
    if (!browser) {
      return;
    }

    localStorage.setItem(key, JSON.stringify(value));
  }
}
