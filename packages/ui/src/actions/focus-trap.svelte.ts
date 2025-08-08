import { createFocusTrap } from 'focus-trap';
import type { Options } from 'focus-trap';
import type { Action } from 'svelte/action';

export const focusTrap: Action<HTMLElement, Options | undefined> = (element, options) => {
  $effect(() => {
    const trap = createFocusTrap(element, options);

    trap.activate();

    return () => {
      trap.deactivate();
    };
  });
};
