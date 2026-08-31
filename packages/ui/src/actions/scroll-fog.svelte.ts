import { css } from '@typie/styled-system/css';
import { smootherstep } from '../utils/number';
import type { Action } from 'svelte/action';

const DEFAULT_SIZE = 24;
const EDGE_EPSILON = 1;
const SAMPLES = 6;

const scrollFogClasses = css({
  transition: '[none]',
  "&[data-scroll-fog-ready='true']": {
    transitionProperty: '[--scroll-fog-leading, --scroll-fog-trailing]',
    transitionDuration: '160ms',
    transitionTimingFunction: 'ease-out',
    _motionReduce: { transitionDuration: '0ms' },
  },
}).split(' ');

const AXES = {
  horizontal: {
    contentExtent: 'scrollWidth',
    direction: 'right',
    position: 'scrollLeft',
    viewportExtent: 'clientWidth',
  },
  vertical: {
    contentExtent: 'scrollHeight',
    direction: 'bottom',
    position: 'scrollTop',
    viewportExtent: 'clientHeight',
  },
} as const;

export type ScrollFogOptions = {
  orientation: keyof typeof AXES;
  size?: number;
};

const normalizedSize = (size: number | undefined) => {
  if (size === undefined || !Number.isFinite(size)) return DEFAULT_SIZE;
  return Math.max(0, size);
};

const maskImage = (direction: 'right' | 'bottom', size: number) => {
  if (size === 0) return `linear-gradient(to ${direction}, black, black)`;

  const leading = Array.from({ length: SAMPLES + 1 }, (_, index) => {
    const progress = index / SAMPLES;
    const alpha = smootherstep(progress);
    return `rgb(0 0 0 / calc(1 - var(--scroll-fog-leading) * ${1 - alpha})) ${progress * size}px`;
  });
  const trailing = Array.from({ length: SAMPLES + 1 }, (_, index) => {
    const progress = index / SAMPLES;
    const alpha = smootherstep(progress);
    return `rgb(0 0 0 / calc(1 - var(--scroll-fog-trailing) * ${alpha})) calc(100% - ${size - progress * size}px)`;
  });

  return `linear-gradient(to ${direction}, ${leading.join(', ')}, black ${size}px, black calc(100% - ${size}px), ${trailing.join(', ')})`;
};

export const scrollFog: Action<HTMLElement, ScrollFogOptions> = (element, initialOptions) => {
  let options = initialOptions;
  let currentMaskKey: string | undefined;
  let transitionFrame: number | undefined;
  const previousMaskImage = element.style.maskImage;
  const addedClasses = scrollFogClasses.filter((className) => !element.classList.contains(className));

  element.classList.add(...addedClasses);

  const sync = () => {
    const axis = AXES[options.orientation];
    const viewportExtent = element[axis.viewportExtent];
    const maximum = Math.max(0, element[axis.contentExtent] - viewportExtent);
    const position = Math.max(0, element[axis.position]);
    const effectiveSize = Math.min(normalizedSize(options.size), viewportExtent / 2);
    const leading = position > EDGE_EPSILON;
    const trailing = position < maximum - EDGE_EPSILON;
    const maskKey = `${options.orientation}:${effectiveSize}`;

    element.style.setProperty('--scroll-fog-leading', leading ? '1' : '0');
    element.style.setProperty('--scroll-fog-trailing', trailing ? '1' : '0');
    if (maskKey !== currentMaskKey) {
      currentMaskKey = maskKey;
      element.style.maskImage = maskImage(axis.direction, effectiveSize);
    }
  };

  const resizeObserver = new ResizeObserver(sync);
  const observeCurrentElements = () => {
    resizeObserver.disconnect();
    resizeObserver.observe(element);
    for (const child of element.children) resizeObserver.observe(child);
  };
  const mutationObserver = new MutationObserver(() => {
    observeCurrentElements();
    sync();
  });

  observeCurrentElements();
  mutationObserver.observe(element, { childList: true });
  element.addEventListener('scroll', sync);
  sync();
  transitionFrame = requestAnimationFrame(() => {
    sync();
    transitionFrame = requestAnimationFrame(() => {
      sync();
      transitionFrame = undefined;
      element.dataset.scrollFogReady = 'true';
    });
  });

  return {
    update(nextOptions) {
      options = nextOptions;
      sync();
    },
    destroy() {
      element.removeEventListener('scroll', sync);
      mutationObserver.disconnect();
      resizeObserver.disconnect();
      if (transitionFrame !== undefined) cancelAnimationFrame(transitionFrame);
      delete element.dataset.scrollFogReady;
      element.classList.remove(...addedClasses);
      element.style.removeProperty('--scroll-fog-leading');
      element.style.removeProperty('--scroll-fog-trailing');
      element.style.maskImage = previousMaskImage;
    },
  };
};
