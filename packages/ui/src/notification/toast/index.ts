import { toast as sonner } from 'svelte-sonner';
import Item from './Item.svelte';
import PromiseItem from './PromiseItem.svelte';

export type ToastOptions = {
  description?: string;
  duration?: number;
};

export type PromiseToastMessages<T> = {
  loading: string;
  success: string | ((data: T) => string);
  error: string | ((error: unknown) => string);
};

export type PromiseToastOptions = {
  duration?: number;
};

const add = (_: 'success' | 'error', message: string, options?: ToastOptions) => {
  sonner.custom(Item, {
    componentProps: {
      message,
      description: options?.description,
    },

    duration: options?.duration ?? 3000,
  });
};

const resolve = <T>(message: string | ((data: T) => string), data: T): string => {
  return typeof message === 'function' ? message(data) : message;
};

export const toast = {
  success: (message: string, options?: ToastOptions) => add('success', message, options),
  error: (message: string, options?: ToastOptions) => add('error', message, options),

  promise: <T>(promise: Promise<T>, messages: PromiseToastMessages<T>, options?: PromiseToastOptions): Promise<T> => {
    const duration = options?.duration ?? 3000;

    const id = sonner.custom(PromiseItem, {
      componentProps: { message: messages.loading, loading: true },
      duration: Infinity,
      dismissible: false,
    });

    promise.then(
      (data) => {
        sonner.custom(PromiseItem, {
          id,
          componentProps: { message: resolve(messages.success, data), loading: false },
          duration,
        });
      },
      (err) => {
        sonner.custom(PromiseItem, {
          id,
          componentProps: { message: resolve(messages.error, err), loading: false },
          duration,
        });
      },
    );

    return promise;
  },
};
