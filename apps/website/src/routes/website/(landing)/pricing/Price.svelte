<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import { tweened } from '$lib/landing/tween.svelte';

  type Props = { value: number; unit: string };

  let { value, unit }: Props = $props();

  const DURATION_MS = 280;

  const shown = tweened(value);

  const reduced = $derived(prefersReducedMotion.current);
  const text = $derived(comma(Math.round(shown.current)));

  $effect(() => {
    const to = value;
    const from = untrack(() => shown.current);
    if (reduced || from === to) {
      shown.set(to);
      return;
    }
    shown.to(from, to, DURATION_MS);
    return shown.cancel;
  });

  const numberClass = css({
    fontFamily: 'landing',
    fontSize: { base: '[32px]', md: '[40px]' },
    fontWeight: 'bold',
    lineHeight: '[1]',
    letterSpacing: '[-0.03em]',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.default',
  });
  const unitClass = css({
    marginLeft: '[0.12em]',
    fontSize: '[0.28em]',
    fontWeight: 'semibold',
    letterSpacing: '[-0.01em]',
    color: 'text.muted',
  });
</script>

<span class={numberClass}>
  {text}
  <span class={unitClass}>{unit}</span>
</span>
