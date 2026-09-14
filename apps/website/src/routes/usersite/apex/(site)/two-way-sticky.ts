import type { Action } from 'svelte/action';

type Parameter = {
  headerBottom: number;
  viewportHeight: number;
};

const EDGE = 28;

export const twoWaySticky: Action<HTMLElement, Parameter> = (node, parameter) => {
  let headerBottom = parameter.headerBottom;
  let viewportHeight = parameter.viewportHeight;
  let lastY = window.scrollY;
  let frame = 0;

  const apply = () => {
    const y = window.scrollY;
    const dy = y - lastY;
    lastY = y;

    if (getComputedStyle(node).position !== 'sticky') {
      node.style.top = '';
      return;
    }

    const maxTop = headerBottom + EDGE;
    const minTop = Math.min(maxTop, viewportHeight - node.offsetHeight - EDGE);
    const current = Number.parseFloat(node.style.top);
    const base = Number.isNaN(current) ? maxTop : current;
    node.style.top = `${Math.min(maxTop, Math.max(minTop, base - dy))}px`;
  };

  const onscroll = () => {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      apply();
    });
  };

  const observer = new ResizeObserver(() => apply());
  observer.observe(node);

  window.addEventListener('scroll', onscroll, { passive: true });
  apply();

  return {
    update(next) {
      headerBottom = next.headerBottom;
      viewportHeight = next.viewportHeight;
      apply();
    },
    destroy() {
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener('scroll', onscroll);
    },
  };
};
