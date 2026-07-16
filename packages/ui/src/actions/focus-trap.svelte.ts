import { createFocusTrap } from 'focus-trap';
import type { Options } from 'focus-trap';
import type { Action } from 'svelte/action';

const focusTrapMap = new WeakMap<HTMLElement, ReturnType<typeof createFocusTrap>>();

export const deactivateFocusTrap = (element: HTMLElement, options?: { returnFocus?: boolean }) => {
  focusTrapMap.get(element)?.deactivate(options);
};

export const updateFocusTrapContainers = (element: HTMLElement, containers: HTMLElement[]) => {
  focusTrapMap.get(element)?.updateContainerElements(containers);
};

export const focusTrap: Action<HTMLElement, Options | undefined> = (element, options) => {
  $effect(() => {
    const trap = createFocusTrap(element, options);
    focusTrapMap.set(element, trap);

    trap.activate();

    return () => {
      trap.deactivate();
      focusTrapMap.delete(element);
    };
  });
};
