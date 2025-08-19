import type * as Y from 'yjs';

export class YState<T> {
  #current = $state<T>() as T;

  #doc: Y.Doc;
  #map: Y.Map<T>;
  #name: string;

  constructor(doc: Y.Doc, name: string, defaultValue: T) {
    this.#doc = doc;
    this.#name = name;

    this.#map = doc.getMap('attrs');

    this.#current = this.#map.get(name) ?? defaultValue;

    const handler = () => {
      this.#current = this.#map.get(name) ?? defaultValue;
    };

    $effect(() => {
      this.#map.observe(handler);
      return () => {
        this.#map.unobserve(handler);
      };
    });
  }

  get current() {
    return this.#current;
  }

  set current(value: T) {
    this.#current = value;

    this.#doc.transact(() => {
      this.#map.set(this.#name, value);
    }, 'local');
  }
}
