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

    let lastWidth = 0;

    const handler = () => {
      // NOTE: 요소가 숨겨져 있으면 scrollHeight 계산을 건너뜀
      if (element.offsetParent === null) {
        return;
      }

      element.style.height = 'auto';
      const height = element.scrollHeight;
      element.style.height = `${height}px`;

      if (cacheKey) {
        heightCache.set(cacheKey, height);
      }
    };

    const off = on(element, 'input', handler);

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const newWidth = entry.contentRect.width;

        if (newWidth > 0 && newWidth !== lastWidth) {
          lastWidth = newWidth;
          requestAnimationFrame(() => {
            handler();
          });
        }
      }
    });

    resizeObserver.observe(element);

    requestAnimationFrame(() => {
      handler();
    });

    return () => {
      off();
      resizeObserver.disconnect();
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
