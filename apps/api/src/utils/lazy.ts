export class Lazy<T> {
  #value: T | undefined;
  #fn: () => Promise<T>;

  constructor(fn: () => Promise<T>) {
    this.#fn = fn;
    this.#value = undefined;
  }

  async get(): Promise<T> {
    if (this.#value === undefined) {
      this.#value = await this.#fn();
    }

    return this.#value;
  }
}
