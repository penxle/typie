import { z } from 'zod';
import type { Awaitable } from '../utils';

type FormFieldErrors<T> = {
  [K in keyof T]: string | undefined;
};

type FormState = {
  isLoading: boolean;
  isDirty: boolean;
};

type CreateFormOptions<Schema extends z.ZodTypeAny, D extends Partial<z.infer<Schema>>> = {
  schema: Schema;
  defaultValues?: D;
  submitOn?: 'change' | 'submit';
  onSubmit: (data: z.infer<Schema>) => Awaitable<void>;
  onError?: (error: unknown) => void;
};

export class Form<Schema extends z.ZodTypeAny, D extends Partial<z.infer<Schema>>> {
  fields: z.infer<Schema>;

  #formState = $state<FormState>({ isLoading: false, isDirty: false });
  #formData = $state<Partial<z.infer<Schema>>>({});
  #errors = $state<FormFieldErrors<z.infer<Schema>>>({} as FormFieldErrors<z.infer<Schema>>);
  #defaultValues: Partial<z.infer<Schema>>;

  #options: CreateFormOptions<Schema, D>;

  get errors() {
    return this.#errors;
  }

  get state() {
    return this.#formState;
  }

  constructor(options: CreateFormOptions<Schema, D>) {
    this.#options = options;
    this.#defaultValues = options.defaultValues ?? {};
    this.#formData = { ...this.#defaultValues };

    this.fields = new Proxy(this.#formData, {
      set: (target, prop, value) => {
        target[prop as keyof z.infer<Schema>] = value;

        this.#formState.isDirty = true;

        if (this.#options.submitOn === 'change') {
          this.handleSubmit();
        }

        return true;
      },
    }) as z.infer<Schema>;
  }

  handleSubmit = async (event?: SubmitEvent) => {
    event?.preventDefault();

    this.#formState.isLoading = true;
    try {
      const data = this.#options.schema.parse(this.#formData) as z.infer<Schema>;
      for (const key of Object.keys(this.#errors)) {
        this.#errors[key as keyof z.infer<Schema>] = undefined;
      }

      try {
        await this.#options.onSubmit(data);
      } catch (err) {
        this.#options.onError?.(err);
        throw err;
      }
    } catch (err) {
      const errors = {} as FormFieldErrors<z.infer<Schema>>;

      if (err instanceof FormError) {
        errors[err.field as keyof z.infer<Schema>] = err.message;
      } else if (err instanceof z.ZodError) {
        const { fieldErrors } = err.flatten();
        for (const [key, value] of Object.entries(fieldErrors)) {
          errors[key as keyof z.infer<Schema>] = Array.isArray(value) ? value[0] : value;
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

  reset = () => {
    Object.keys(this.#formData).forEach((key) => {
      this.#formData[key as keyof z.infer<Schema>] = this.#defaultValues[key as keyof z.infer<Schema>];
    });

    for (const key of Object.keys(this.#errors)) {
      this.#errors[key as keyof z.infer<Schema>] = undefined;
    }

    this.#formState.isDirty = false;
    this.#formState.isLoading = false;
  };
}

export const createForm = <Schema extends z.ZodTypeAny, D extends Partial<z.infer<Schema>> = Partial<z.infer<Schema>>>(
  options: CreateFormOptions<Schema, D>,
): Form<Schema, D> => {
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
