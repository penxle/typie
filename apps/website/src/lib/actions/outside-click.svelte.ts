import { on } from 'svelte/events';
import type { Action } from 'svelte/action';

export const outsideClick: Action<HTMLElement, undefined, { onoutsideclick: () => void }> = (element) => {
  $effect(() => {
    let onclick: (() => void) | undefined;

    setTimeout(() => {
      onclick = on(window, 'click', (event) => {
        if (!element.contains(event.target as Node)) {
          element.dispatchEvent(new CustomEvent('outsideclick'));
        }
      });
    });

    return () => {
      onclick?.();
    };
  });
};
