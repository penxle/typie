import { z } from 'zod';
import type { Awaitable } from '$lib/utils';

type ZodShape<T> = {
  [K in keyof T]: z.ZodTypeAny;
};

type FormFields<T, D extends Partial<T>> = {
  [K in keyof T]: K extends keyof D ? (D[K] extends undefined ? T[K] | undefined : D[K]) : T[K] | undefined;
};

type FormFieldErrors<T> = {
  [K in keyof T]: string | undefined;
};

type FormState = {
  isLoading: boolean;
};

type CreateFormOptions<T, D extends Partial<T>> = {
  schema:
    | z.ZodObject<ZodShape<T>, z.UnknownKeysParam, z.ZodTypeAny, T>
    | z.ZodEffects<z.ZodObject<ZodShape<T>, z.UnknownKeysParam, z.ZodTypeAny, T>>;
  defaultValues?: D;
  onSubmit: (data: T) => Awaitable<void>;
  onError?: (error: unknown) => void;
};

type CreateFormReturn<T, D extends Partial<T>> = {
  handleSubmit: (event: SubmitEvent) => Promise<void>;
  fields: FormFields<T, D>;
  errors: FormFieldErrors<T>;
} & FormState;

export const createForm = <T extends Record<string, unknown>, D extends Partial<T>>(
  options: CreateFormOptions<T, D>,
): CreateFormReturn<T, D> => {
  const formState = $state<FormState>({
    isLoading: false,
  });

  const formData = $state<Partial<T>>(options.defaultValues ?? {});
  const errors = $state<FormFieldErrors<T>>({} as FormFieldErrors<T>);

  const handleSubmit = async (event: SubmitEvent) => {
    event.preventDefault();

    formState.isLoading = true;
    try {
      const data = options.schema.parse(formData);
      for (const key of Object.keys(errors)) {
        errors[key as keyof T] = undefined;
      }

      try {
        await options.onSubmit(data);
      } catch (err) {
        options.onError?.(err);
        throw err;
      }
    } catch (err) {
      const erroredFields: string[] = [];

      if (err instanceof FormError) {
        errors[err.field as keyof T] = err.message;
        erroredFields.push(err.field);
      } else if (err instanceof z.ZodError) {
        const { fieldErrors } = err.flatten();
        for (const [key, value] of Object.entries(fieldErrors)) {
          errors[key as keyof T] = Array.isArray(value) ? value[0] : value;
          erroredFields.push(key);
        }
      } else {
        throw err;
      }

      for (const key of Object.keys(errors)) {
        if (!erroredFields.includes(key)) {
          errors[key as keyof T] = undefined;
        }
      }
    } finally {
      formState.isLoading = false;
    }
  };

  const fields = new Proxy(formData, {
    set: (target, prop, value) => {
      target[prop as keyof T] = value;
      return true;
    },
  }) as FormFields<T, D>;

  const form = $derived({
    fields,
    errors,
    handleSubmit,
    ...formState,
  });

  return form;
};

export class FormError extends Error {
  field: string;

  constructor(field: string, message: string) {
    super(message);

    this.name = 'FormError';
    this.field = field;
  }
}
