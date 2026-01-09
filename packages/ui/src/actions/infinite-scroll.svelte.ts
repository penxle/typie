import type { Action } from 'svelte/action';

type Parameter = {
  onLoadMore: () => void;
  enabled?: boolean;
  rootMargin?: string;
};

export const infiniteScroll: Action<HTMLElement, Parameter> = (element, params) => {
  $effect(() => {
    const { onLoadMore, enabled = true, rootMargin = '100px' } = params;

    if (!enabled) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          onLoadMore();
        }
      },
      { rootMargin },
    );

    observer.observe(element);

    return () => {
      observer.disconnect();
    };
  });
};
