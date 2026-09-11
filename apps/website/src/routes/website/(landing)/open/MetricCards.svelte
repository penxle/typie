<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import Change from './Change.svelte';
  import { formatWithUnit, METRICS } from './open';
  import Sparkline from './Sparkline.svelte';
  import type { OpenStats } from './open';

  type Props = { stats: OpenStats };

  let { stats }: Props = $props();

  const gridClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', md: '[repeat(2, minmax(0, 1fr))]', lg: '[repeat(3, minmax(0, 1fr))]' },
    gap: '16px',
    marginTop: { base: '28px', md: '40px' },
  });
  const cardClass = css({
    display: 'grid',
    alignContent: 'space-between',
    gap: '24px',
    padding: { base: '24px', md: '28px' },
    borderRadius: '[16px]',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    transition: '[border-color 0.2s ease-out]',
    _hover: { borderColor: 'text.default/20' },
  });
  const valueClass = css({
    fontSize: { base: '[36px]', md: '[40px]' },
    fontWeight: 'bold',
    lineHeight: '[1]',
    letterSpacing: '[-0.03em]',
    fontVariantNumeric: 'tabular-nums',
  });
  const titleClass = css({ marginTop: '12px', fontSize: '14px', fontWeight: 'medium', color: 'text.muted' });
  const footClass = css({ display: 'grid', gap: '12px' });
  const sparkHostClass = css({ '& svg': { width: 'full' } });
</script>

<div class={gridClass}>
  {#each METRICS as metric (metric.key)}
    {@const series = stats[metric.key]}
    <div class={cardClass}>
      <div>
        <p class={valueClass}>{formatWithUnit(series.current, metric.unit)}</p>
        <h3 class={titleClass}>{metric.title}</h3>
      </div>
      <div class={footClass}>
        <div class={sparkHostClass}><Sparkline data={series.data} height={32} tone="muted" width={240} /></div>
        <Change data={series.data} kind={metric.kind} />
      </div>
    </div>
  {/each}
</div>
