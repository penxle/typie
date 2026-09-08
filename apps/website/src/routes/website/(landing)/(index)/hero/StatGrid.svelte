<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { HIDDEN_BELOW, rise } from '../scrub';
  import Odometer from './Odometer.svelte';
  import Reveal from './Reveal.svelte';

  export type StatItem = { value: number; unit: string; label: string; rate?: number };

  type Props = { items: readonly StatItem[]; shown: boolean; sideProgress?: number };

  let { items, shown, sideProgress = 0 }: Props = $props();

  const CENTER_DELAY = 200;
  const ROW_GAP = 28;

  const center = $derived(Math.floor(items.length / 2));

  const gridClass = css({
    position: 'relative',
    display: 'grid',
    gridTemplateColumns: { base: '1fr', md: '[repeat(3, minmax(0, 1fr))]' },
    rowGap: '28px',
    width: 'full',
    maxWidth: '[1000px]',
    marginX: 'auto',
  });
  const cellClass = css({
    display: 'grid',
    justifyItems: 'center',
    '&[data-fold]': {
      position: { base: 'absolute', md: 'static' },
      left: { base: '0', md: '[auto]' },
      right: { base: '0', md: '[auto]' },
    },
    '&[data-fold="before"]': { bottom: { base: '[calc(100% + var(--row-gap))]', md: '[auto]' } },
    '&[data-fold="after"]': { top: { base: '[calc(100% + var(--row-gap))]', md: '[auto]' } },
  });
  const sideClass = css({ willChange: 'opacity, transform' });
  const stackClass = css({ display: 'inline-flex', flexDirection: 'column', alignItems: 'center' });
  const labelClass = css({
    marginTop: '[calc(12px - 0.267em)]',
    marginBottom: '[-0.264em]',
    fontFamily: 'landing',
    fontSize: '13px',
    color: 'text.hint',
    whiteSpace: 'nowrap',
  });
</script>

{#snippet stat(item: StatItem)}
  <div class={stackClass}>
    <Odometer rate={item.rate ?? 0} unit={item.unit} value={item.value} />
    <span class={labelClass}>{item.label}</span>
  </div>
{/snippet}

<div style:--row-gap="{ROW_GAP}px" class={gridClass}>
  {#each items as item, index (item.label)}
    {#if index === center}
      <div class={cellClass}>
        <Reveal delay={CENTER_DELAY} {shown}>
          {@render stat(item)}
        </Reveal>
      </div>
    {:else}
      <div class={cellClass} data-fold={index < center ? 'before' : 'after'}>
        <div
          style:opacity={sideProgress}
          style:transform={rise(sideProgress, 12)}
          class={sideClass}
          aria-hidden={sideProgress < HIDDEN_BELOW}
        >
          {@render stat(item)}
        </div>
      </div>
    {/if}
  {/each}
</div>
