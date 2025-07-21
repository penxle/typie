import { MotionValue, motionValue } from 'motion';

export class Motion<T> {
  #current = $state() as T;
  #value: MotionValue<T>;

  constructor(value: T) {
    this.#current = value;
    this.#value = motionValue(value);

    $effect(() => {
      return this.#value.on('change', (value) => {
        this.#current = value;
      });
    });
  }

  get current() {
    return this.#current;
  }

  get value() {
    return this.#value;
  }
}
