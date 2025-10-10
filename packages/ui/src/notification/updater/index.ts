import { toast as sonner } from 'svelte-sonner';
import Item from './Item.svelte';

export type UpdaterOptions = {
  onRefresh?: () => void;
};

const show = (options?: UpdaterOptions) => {
  const existing = sonner.getActiveToasts().filter((toast) => toast.id === 'updater');
  if (existing.length > 0) {
    return;
  }

  sonner.custom(Item, {
    id: 'updater',
    componentProps: {
      onRefresh: options?.onRefresh,
    },
    duration: Infinity,
  });
};

export const updater = {
  show: (options?: UpdaterOptions) => show(options),
};
