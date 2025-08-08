import { untrack } from 'svelte';
import { SvelteMap, SvelteURLSearchParams } from 'svelte/reactivity';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { debounce } from '../utils';

const pendingUpdates = new SvelteMap<string, string | null>();

$effect.root(() => {
  $effect(() => {
    if (pendingUpdates.size > 0) {
      untrack(() => {
        const params = new SvelteURLSearchParams(page.url.searchParams);

        for (const [key, value] of pendingUpdates) {
          if (value === null) {
            params.delete(key);
          } else {
            params.set(key, value);
          }
        }

        const queryString = params.toString();
        const newUrl = `${page.url.pathname}${queryString ? `?${queryString}` : ''}`;

        if (newUrl === page.url.pathname + page.url.search) {
          pendingUpdates.clear();
        } else {
          goto(newUrl, { replaceState: true, keepFocus: true }).then(() => {
            pendingUpdates.clear();
          });
        }
      });
    }
  });
});

type QueryStringOptions = {
  debounce?: number;
};

export class QueryString<T = string> {
  #key: string;
  #debounce: number;

  #parse: (value: string | null) => T;
  #stringify: (value: T) => string | null;

  #update: () => void;
  #current = $state<T>() as T;

  constructor(
    key: string,
    defaultValue: T,
    options: QueryStringOptions & {
      parse?: (value: string | null) => T;
      stringify?: (value: T) => string | null;
    } = {},
  ) {
    // setupEffect();

    this.#key = key;
    this.#debounce = options.debounce ?? 0;

    this.#parse = options.parse ?? ((value) => (value ?? defaultValue) as T);
    this.#stringify =
      options.stringify ??
      ((value) => {
        if (value === defaultValue || value === null || value === undefined || value === '') {
          return null;
        }

        return String(value);
      });

    const urlValue = page.url.searchParams.get(key);
    this.#current = this.#parse(urlValue);

    const update = () => pendingUpdates.set(this.#key, this.#stringify(this.#current));
    this.#update = this.#debounce > 0 ? debounce(update, this.#debounce) : update;

    $effect(() => {
      const newUrlValue = page.url.searchParams.get(this.#key);
      const newValue = this.#parse(newUrlValue);

      untrack(() => {
        if (this.#current !== newValue) {
          this.#current = newValue;
        }
      });
    });
  }

  get current(): T {
    return this.#current;
  }

  set current(value: T) {
    if (this.#current !== value) {
      this.#current = value;
      this.#update();
    }
  }
}

export class QueryStringNumber extends QueryString<number> {
  constructor(key: string, defaultValue = 0, options?: QueryStringOptions) {
    super(key, defaultValue, {
      ...options,
      parse: (value) => {
        if (value === null) return defaultValue;
        const num = Number(value);
        return Number.isNaN(num) ? defaultValue : num;
      },
      stringify: (value) => {
        if (value === defaultValue) return null;
        return value.toString();
      },
    });
  }
}

export class QueryStringBoolean extends QueryString<boolean> {
  constructor(key: string, defaultValue = false, options?: QueryStringOptions) {
    super(key, defaultValue, {
      ...options,
      parse: (value) => value === 'true',
      stringify: (value) => {
        if (value === defaultValue) return null;
        return value ? 'true' : 'false';
      },
    });
  }

  toggle() {
    this.current = !this.current;
  }
}

export class QueryStringArray<T extends string = string> extends QueryString<T[]> {
  constructor(key: string, defaultValue: T[] = [], options?: QueryStringOptions) {
    super(key, defaultValue, {
      ...options,
      parse: (value) => {
        if (!value) return defaultValue;
        return value.split(',').filter(Boolean) as T[];
      },
      stringify: (value) => {
        if (value.length === 0 || JSON.stringify(value) === JSON.stringify(defaultValue)) return null;
        return value.join(',');
      },
    });
  }

  add(item: T) {
    if (!this.current.includes(item)) {
      this.current = [...this.current, item];
    }
  }

  remove(item: T) {
    this.current = this.current.filter((i) => i !== item);
  }

  toggle(item: T) {
    if (this.current.includes(item)) {
      this.remove(item);
    } else {
      this.add(item);
    }
  }
}
