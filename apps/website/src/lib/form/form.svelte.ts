import { z } from 'zod';
import type { Awaitable } from '$lib/utils';

type ZodShape<T> = {
  [K in keyof T]: z.ZodTypeAny;
};

type ExactPartial<T, Shape> = {
  [K in keyof T]: K extends keyof Shape ? T[K] : never;
} & Partial<Shape>;

type IsStringLiteral<T> = T extends string ? (string extends T ? false : true) : false;

type FormField<T, D, K extends keyof T> = K extends keyof D
  ? D[K] extends undefined
    ? T[K] | undefined
    : IsStringLiteral<T[K]> extends true
      ? T[K]
      : D[K]
  : T[K] | undefined;

type FormFields<T, D extends Partial<T>> = {
  [K in keyof T]: FormField<T, D, K>;
};

type FormFieldErrors<T> = {
  [K in keyof T]: string | undefined;
};

type FormState = {
  isLoading: boolean;
  isDirty: boolean;
};

type CreateFormOptions<T, D extends Partial<T>> = {
  schema:
    | z.ZodObject<ZodShape<T>, z.UnknownKeysParam, z.ZodTypeAny, T>
    | z.ZodEffects<z.ZodObject<ZodShape<T>, z.UnknownKeysParam, z.ZodTypeAny, T>>;
  defaultValues?: ExactPartial<D, T>;
  submitOn?: 'change' | 'submit';
  onSubmit: (data: T) => Awaitable<void>;
  onError?: (error: unknown) => void;
};

export class Form<T extends Record<string, unknown>, D extends Partial<T>> {
  fields: FormFields<T, D>;

  #formState = $state<FormState>({ isLoading: false, isDirty: false });
  #formData = $state<Partial<T>>({});
  #errors = $state<FormFieldErrors<T>>({} as FormFieldErrors<T>);

  #options: CreateFormOptions<T, D>;

  get errors() {
    return this.#errors;
  }

  get state() {
    return this.#formState;
  }

  constructor(options: CreateFormOptions<T, D>) {
    this.#options = options;
    this.#formData = options.defaultValues ?? {};

    this.fields = this.#formData as FormFields<T, D>;

    this.fields = new Proxy(this.#formData, {
      set: (target, prop, value) => {
        target[prop as keyof T] = value;

        this.#formState.isDirty = true;

        if (this.#options.submitOn === 'change') {
          this.handleSubmit();
        }

        return true;
      },
    }) as FormFields<T, D>;
  }

  handleSubmit = async (event?: SubmitEvent) => {
    event?.preventDefault();

    this.#formState.isLoading = true;
    try {
      const data = this.#options.schema.parse(this.#formData);
      for (const key of Object.keys(this.#errors)) {
        this.#errors[key as keyof T] = undefined;
      }

      try {
        await this.#options.onSubmit(data);
      } catch (err) {
        this.#options.onError?.(err);
        throw err;
      }
    } catch (err) {
      const errors = {} as FormFieldErrors<T>;

      if (err instanceof FormError) {
        errors[err.field as keyof T] = err.message;
      } else if (err instanceof z.ZodError) {
        const { fieldErrors } = err.flatten();
        for (const [key, value] of Object.entries(fieldErrors)) {
          errors[key as keyof T] = Array.isArray(value) ? value[0] : value;
        }
      } else {
        throw err;
      }

      this.#errors = errors;
    } finally {
      this.#formState.isLoading = false;
      this.#formState.isDirty = false;
    }
  };
}

export const createForm = <T extends Record<string, unknown>, D extends Partial<T>>(options: CreateFormOptions<T, D>): Form<T, D> => {
  return new Form(options);
};

export class FormError extends Error {
  field: string;

  constructor(field: string, message: string) {
    super(message);

    this.name = 'FormError';
    this.field = field;
  }
}
