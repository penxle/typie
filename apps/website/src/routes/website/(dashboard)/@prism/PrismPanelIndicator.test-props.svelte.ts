class ReactiveProps<T extends object> {
  value: T = $state() as T;

  constructor(value: T) {
    this.value = value;
  }
}

export const reactiveProps = <T extends object>(props: T): T => new ReactiveProps(props).value;
