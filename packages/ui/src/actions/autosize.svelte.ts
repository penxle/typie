import { on } from 'svelte/events';
import type { Action } from 'svelte/action';

export const autosize: Action<HTMLTextAreaElement> = (element) => {
  $effect(() => {
    element.style.overflow = 'hidden';

    const handler = () => {
      element.style.height = 'auto';
      element.style.height = `${element.scrollHeight}px`;
    };

    const off = on(element, 'input', handler);

    requestAnimationFrame(() => {
      handler();
    });

    return () => {
      off();
    };
  });
};
