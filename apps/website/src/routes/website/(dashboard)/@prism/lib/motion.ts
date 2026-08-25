import { quintOut } from 'svelte/easing';
import { fade, fly, scale, slide } from 'svelte/transition';
import type { TransitionConfig } from 'svelte/transition';

export const MOTION = { quick: 100, state: 150, enter: 200, expand: 200, block: 280 } as const;
export const PRISM_VISIBILITY_MOTION = {
  duration: 400,
  hiddenScale: 0.94,
  easing: 'cubic-bezier(0.32, 0.72, 0, 1)',
} as const;

export const reducedMotion = (): boolean => typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const cubic = (at: number, first: number, second: number): number => {
  const c = 3 * first;
  const b = 3 * (second - first) - c;
  const a = 1 - c - b;
  return ((a * at + b) * at + c) * at;
};

export const prismVisibilityEasing = (progress: number): number => {
  const x = Math.max(0, Math.min(1, progress));
  let lower = 0;
  let upper = 1;
  let at = x;

  for (let step = 0; step < 24; step += 1) {
    const estimate = cubic(at, 0.32, 0);
    if (Math.abs(estimate - x) < 1e-8) break;
    if (estimate < x) lower = at;
    else upper = at;
    at = (lower + upper) / 2;
  }

  return cubic(at, 0.72, 1);
};

export const fadeIn = { duration: MOTION.state, easing: quintOut };
export const fadeOut = { duration: MOTION.quick, easing: quintOut };

type RiseOptions = { skip?: boolean; delay?: number; block?: boolean };

export const rise = (node: Element, { skip = false, delay = 0, block = false }: RiseOptions = {}): TransitionConfig => {
  if (skip) return { duration: 0 };
  const duration = block ? MOTION.block : MOTION.enter;
  return reducedMotion()
    ? fade(node, { duration, easing: quintOut, delay })
    : fly(node, { y: block ? 10 : 4, delay, duration, easing: quintOut });
};

export const shift = (node: Element, { dir }: { dir: number }): TransitionConfig =>
  reducedMotion() ? fade(node, fadeIn) : fly(node, { x: 8 * Math.sign(dir), duration: MOTION.enter, easing: quintOut });

export const pop = (node: Element, { out = false }: { out?: boolean } = {}): TransitionConfig =>
  reducedMotion() ? fade(node, out ? fadeOut : fadeIn) : scale(node, { start: 0.8, duration: out ? MOTION.enter : 250, easing: quintOut });

const EXPAND_SPEED = 1.6;
const EXPAND_MAX = 400;

export const expand = (node: Element): TransitionConfig => {
  if (reducedMotion()) return fade(node, fadeIn);
  const height = (node as HTMLElement).offsetHeight;
  const duration = Math.min(EXPAND_MAX, Math.max(MOTION.expand, height / EXPAND_SPEED));
  const base = slide(node, { duration, easing: quintOut });
  return { ...base, css: (t, u) => `${base.css?.(t, u) ?? ''};opacity: ${t};min-height: 0` };
};

const SWAP_LIFT = 6;

export const swap = (node: Element, { box, from }: { box?: HTMLElement; from?: number }): TransitionConfig => {
  const element = node as HTMLElement;
  const reduce = reducedMotion();

  let to = 0;
  if (box !== undefined && from !== undefined) {
    box.style.height = '';
    to = box.offsetHeight;
  }

  const resizing = box !== undefined && from !== undefined && from !== to;

  return {
    duration: reduce ? 0 : MOTION.block,
    easing: quintOut,
    tick: (t, u) => {
      const done = t === 1;

      element.style.opacity = done ? '' : String(t);
      if (!reduce) element.style.transform = done ? '' : `translateY(${SWAP_LIFT * u}px)`;

      if (resizing && box !== undefined) {
        box.style.overflow = done ? '' : 'hidden';
        box.style.height = done ? '' : `${from + (to - from) * t}px`;
      }
    },
  };
};
