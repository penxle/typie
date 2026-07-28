import { flushSync, tick } from 'svelte';

export function createTrackedEffect(effect: () => (() => void) | undefined) {
  let runs = 0;
  const destroy = $effect.root(() => {
    $effect(() => {
      runs += 1;
      return effect();
    });
  });

  return {
    flush: async () => {
      await tick();
      flushSync();
    },
    runs: () => runs,
    destroy,
  };
}
