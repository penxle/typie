import type { Action } from 'svelte/action';

type Parameter = boolean;

const isTouchEnvironment = typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0;

export const touchPanLock: Action<HTMLElement, Parameter> = (_element, initialEnabled = false) => {
  if (!isTouchEnvironment || typeof window === 'undefined') {
    return;
  }

  let enabled = initialEnabled;

  const handleTouchMove = (event: TouchEvent) => {
    if (!enabled || event.touches.length !== 1 || !event.cancelable) {
      return;
    }

    event.preventDefault();
  };

  const attach = () => {
    window.addEventListener('touchmove', handleTouchMove, { passive: false });
  };

  const detach = () => {
    window.removeEventListener('touchmove', handleTouchMove);
  };

  if (enabled) {
    attach();
  }

  return {
    update(nextEnabled = false) {
      if (enabled === nextEnabled) {
        return;
      }

      enabled = nextEnabled;
      if (enabled) {
        attach();
      } else {
        detach();
      }
    },
    destroy() {
      detach();
    },
  };
};
