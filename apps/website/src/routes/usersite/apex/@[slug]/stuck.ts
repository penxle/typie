import type { Action } from 'svelte/action';

type Parameter = {
  onchange?: (node: HTMLElement, stuck: boolean) => void;
};

export const stuck: Action<HTMLElement, Parameter | undefined> = (node, parameter) => {
  let current = parameter;
  let value = false;

  const sentinel = document.createElement('div');
  sentinel.setAttribute('aria-hidden', 'true');
  sentinel.style.height = '0px';
  node.before(sentinel);

  const set = (next: boolean) => {
    if (next === value) return;
    value = next;
    node.toggleAttribute('data-stuck', next);
    current?.onchange?.(node, next);
  };

  const stickyTop = () => Number.parseFloat(getComputedStyle(node).top) || 0;

  let observer: IntersectionObserver | undefined;
  const observe = () => {
    observer?.disconnect();
    const top = Math.ceil(stickyTop());
    observer = new IntersectionObserver(
      ([entry]) => {
        set(node.offsetParent !== null && !entry.isIntersecting && entry.boundingClientRect.top < top);
      },
      { rootMargin: `-${top}px 0px 0px 0px`, threshold: 0 },
    );
    observer.observe(sentinel);
  };

  observe();
  const onresize = () => observe();
  window.addEventListener('resize', onresize);

  return {
    update(next) {
      current = next;
    },
    destroy() {
      window.removeEventListener('resize', onresize);
      observer?.disconnect();
      sentinel.remove();
      set(false);
    },
  };
};
