import { writable } from 'svelte/store';

export type Tip = {
  id: symbol;
  key: string;
  message: string;
};

export const store = writable<Tip[]>([]);
const append = (tip: Omit<Tip, 'id'>) => {
  if (globalThis.window === undefined) {
    throw new TypeError('tip can only be used in browser');
  }

  const tips = JSON.parse(localStorage.getItem('typie:tips') ?? '[]') as string[];
  if (tips.includes(tip.key)) {
    return;
  }

  localStorage.setItem('typie:tips', JSON.stringify([...tips, tip.key]));

  store.update((tips) => [...tips, { id: Symbol(), ...tip }]);
};

export const tip = {
  show: (key: string, message: string) => append({ key, message }),
};
