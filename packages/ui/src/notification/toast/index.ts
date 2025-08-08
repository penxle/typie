import { toast as sonner } from 'svelte-sonner';
import Item from './Item.svelte';

export type ToastOptions = {
  description?: string;
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

export const toast = {
  success: (message: string, options?: ToastOptions) => add('success', message, options),
  error: (message: string, options?: ToastOptions) => add('error', message, options),
};
