import { quartOut } from 'svelte/easing';

export const tweened = (initial: number) => {
  let current = $state(initial);
  let frame = 0;

  const cancel = () => cancelAnimationFrame(frame);

  return {
    get current() {
      return current;
    },
    set(value: number) {
      cancel();
      current = value;
    },
    to(from: number, target: number, ms: number) {
      cancel();
      const start = performance.now();
      const step = (now: number) => {
        const t = Math.min(1, (now - start) / ms);
        current = from + (target - from) * quartOut(t);
        if (t < 1) frame = requestAnimationFrame(step);
      };
      frame = requestAnimationFrame(step);
    },
    cancel,
  };
};
