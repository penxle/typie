<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import { tweened } from '$lib/tween.svelte';

  type Props = { value: number; unit: string; rate?: number };

  let { value, unit, rate = 0 }: Props = $props();

  const START_RATIO = 0.92;
  const INTRO_MS = 2400;
  const TICK_MS = 2000;

  const shown = tweened(0);

  const reduced = $derived(prefersReducedMotion.current);
  const text = $derived(comma(Math.round(shown.current)));

  $effect(() => {
    const end = value;
    const gainPerSecond = rate;
    if (reduced) {
      shown.set(end);
      return;
    }

    let timer = 0;
    let current = end;

    shown.to(Math.floor(end * START_RATIO), end, INTRO_MS);

    if (gainPerSecond > 0) {
      const schedule = () => {
        const wait = TICK_MS * (0.5 + Math.random());
        timer = window.setTimeout(() => {
          const gained = gainPerSecond * (wait / 1000) * (0.5 + Math.random());
          const next = current + gained;
          shown.to(current, next, Math.min(1000, wait * 0.6));
          current = next;
          schedule();
        }, wait);
      };
      schedule();
    }

    return () => {
      shown.cancel();
      clearTimeout(timer);
    };
  });

  const numberClass = css({
    marginTop: '[-0.1356em]',
    marginBottom: '[-0.1438em]',
    fontFamily: 'ui',
    fontSize: { base: '[24px]', md: '[30px]', xl: '[36px]' },
    fontWeight: 'bold',
    lineHeight: '[1]',
    letterSpacing: '[-0.025em]',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.muted',
  });
  const unitClass = css({ marginLeft: '[0.08em]', fontSize: '[0.5em]', fontWeight: 'semibold', color: 'text.hint' });
</script>

<span class={numberClass}>
  {text}
  <span class={unitClass}>{unit}</span>
</span>
