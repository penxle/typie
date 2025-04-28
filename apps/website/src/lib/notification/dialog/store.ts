import { writable } from 'svelte/store';
import type { Snippet } from 'svelte';

type Dialog = {
  id: symbol;

  title: string;
  message: string;

  children?: Snippet;

  action?: 'primary' | 'danger';
  actionLabel?: string;
  actionHandler?: () => void;
};

type Alert = Dialog & {
  type: 'alert';
};

type Confirm = Dialog & {
  type: 'confirm';

  cancelLabel?: string;
  cancelHandler?: () => void;
};

export type AllDialog = Alert | Confirm;

export const store = writable<AllDialog[]>([]);
const append = (dialog: Omit<AllDialog, 'id'>) => {
  if (globalThis.window === undefined) {
    throw new TypeError('dialog can only be used in browser');
  }

  store.update((dialogs) => [...dialogs, { id: Symbol(), ...dialog }]);
};

export const dialog = {
  alert: (options: Omit<Alert, 'id' | 'type'>) => append({ ...options, type: 'alert' }),
  confirm: (options: Omit<Confirm, 'id' | 'type'>) => append({ ...options, type: 'confirm' }),
};
