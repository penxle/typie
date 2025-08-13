import { toast as sonner } from 'svelte-sonner';
import Item from './Item.svelte';

export type TipOptions = {
  description?: string;
};

const append = (key: string, message: string, options?: TipOptions) => {
  if (globalThis.window === undefined) {
    throw new TypeError('tip can only be used in browser');
  }

  const saved = JSON.parse(localStorage.getItem('typie:tips') ?? '[]') as string[];
  if (saved.includes(key)) {
    return;
  }

  localStorage.setItem('typie:tips', JSON.stringify([...saved, key]));

  sonner.custom(Item, {
    id: key,
    componentProps: {
      id: key,
      message,
      description: options?.description,
    },
    duration: Infinity,
  });
};

export const tip = {
  show: (key: string, message: string, options?: TipOptions) => append(key, message, options),
};
