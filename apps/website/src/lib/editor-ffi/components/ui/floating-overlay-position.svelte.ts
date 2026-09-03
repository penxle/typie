import { prefersReducedMotion } from '@typie/ui/state';
import { untrack } from 'svelte';

type FloatingOverlayPositionOptions = {
  element: () => HTMLElement | undefined;
  layoutTop: () => number | undefined;
  presented: () => boolean;
};

const motionDuration = (node: HTMLElement): number => {
  const value = getComputedStyle(node).getPropertyValue('--editor-pane-overlay-motion-duration').trim() || '280ms';
  if (value.endsWith('ms')) return Number.parseFloat(value);
  if (value.endsWith('s')) return Number.parseFloat(value) * 1000;
  return Number.parseFloat(value);
};

export const setupFloatingOverlayPosition = ({ element, layoutTop, presented }: FloatingOverlayPositionOptions): void => {
  let animation: Animation | undefined;
  let previousLayoutTop: number | undefined;

  $effect(() => {
    const node = element();
    const nextLayoutTop = layoutTop();
    if (!node || nextLayoutTop === undefined) return;

    const previous = previousLayoutTop;
    previousLayoutTop = nextLayoutTop;
    if (previous === undefined) return;

    const visualTop = node.getBoundingClientRect().top;
    animation?.cancel();
    animation = undefined;
    const targetTop = node.getBoundingClientRect().top;
    const offset = visualTop - targetTop + previous - nextLayoutTop;
    if (!untrack(() => presented() && !prefersReducedMotion.current) || Math.abs(offset) < 0.5) return;

    const styles = getComputedStyle(node);
    const next = node.animate([{ transform: `translateY(${offset}px)` }, { transform: 'translateY(0)' }], {
      duration: motionDuration(node),
      easing: styles.getPropertyValue('--editor-pane-overlay-motion-easing').trim() || 'cubic-bezier(0.22, 1, 0.36, 1)',
      fill: 'both',
    });
    animation = next;
    next.finished
      .catch(() => null)
      .then(() => {
        if (animation !== next) return;
        animation = undefined;
        next.cancel();
      });
  });

  $effect(() => () => animation?.cancel());
};
