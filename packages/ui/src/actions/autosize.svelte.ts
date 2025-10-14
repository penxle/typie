import { on } from 'svelte/events';
import { SvelteMap } from 'svelte/reactivity';
import type { Action } from 'svelte/action';

// NOTE: animateFlip과의 호환성을 위해 캐시된 높이를 즉시 적용해서 항상 올바른 높이를 유지
const heightCache = new SvelteMap<string, number>();

type AutosizeParams = {
  // NOTE: 높이 캐싱을 위한 stable key
  cacheKey?: string;
};

export const autosize: Action<HTMLTextAreaElement, AutosizeParams | undefined> = (element, params = {}) => {
  const cacheKey = params.cacheKey;

  if (cacheKey) {
    const cachedHeight = heightCache.get(cacheKey);
    if (cachedHeight) {
      element.style.height = `${cachedHeight}px`;
    }
  }

  $effect(() => {
    element.style.overflow = 'hidden';

    const handler = () => {
      element.style.height = 'auto';
      const height = element.scrollHeight;
      element.style.height = `${height}px`;

      if (cacheKey) {
        heightCache.set(cacheKey, height);
      }
    };

    const off = on(element, 'input', handler);

    requestAnimationFrame(() => {
      handler();
    });

    return () => {
      off();
    };
  });

  return {
    destroy() {
      if (cacheKey) {
        heightCache.delete(cacheKey);
      }
    },
  };
};
