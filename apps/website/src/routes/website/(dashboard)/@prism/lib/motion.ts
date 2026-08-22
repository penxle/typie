import { quintOut } from 'svelte/easing';
import { fade, fly, scale, slide } from 'svelte/transition';
import type { TransitionConfig } from 'svelte/transition';

export const MOTION = { quick: 100, state: 150, enter: 200, expand: 200 } as const;

export const reducedMotion = (): boolean => typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const fadeIn = { duration: MOTION.state, easing: quintOut };
export const fadeOut = { duration: MOTION.quick, easing: quintOut };

export const rise = (node: Element, { skip = false, delay = 0 }: { skip?: boolean; delay?: number } = {}): TransitionConfig => {
  if (skip) return { duration: 0 };
  return reducedMotion() ? fade(node, { ...fadeIn, delay }) : fly(node, { y: 4, delay, duration: MOTION.enter, easing: quintOut });
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
  return { ...base, css: (t, u) => `${base.css?.(t, u) ?? ''};opacity: ${t}` };
};
