export class Ref<T> {
  current: T;

  constructor(value: T) {
    this.current = value;
  }
}
