import { prefersReducedMotion } from '@typie/ui/state';

type Options = { length: number; shown: boolean; step: number };

export const revealCount = (options: () => Options) => {
  let count = $state(0);

  $effect(() => {
    const { length, shown, step } = options();
    if (!shown) return;

    if (prefersReducedMotion.current) {
      count = length;
      return;
    }

    count = 0;
    let index = 0;
    let timer = window.setTimeout(function tick() {
      index += 1;
      count = index;
      if (index < length) timer = window.setTimeout(tick, step);
    }, step);

    return () => window.clearTimeout(timer);
  });

  return {
    get current() {
      return count;
    },
  };
};
