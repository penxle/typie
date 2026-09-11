<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { ramp, SCENE_LEAD_RANGE, SCENE_SLIDE_RANGE } from '../../scrub';
  import SceneLead from '../SceneLead.svelte';
  import { CELLS, DAYS, FILL_RANGE, MONTH_BLOCKS, MONTH_SPANS } from './activity';
  import type { SceneProps } from '../../scrub';

  let { progress, cardsProgress = 0, lead, cards }: SceneProps = $props();

  const EDGE = 6;
  const LEAD_GAP = 40;
  const TRACK = 1000;

  let leadHeight = $state(0);

  const fill = $derived(Math.round(ramp(progress, ...FILL_RANGE) * DAYS * 4) / 4);
  const slide = $derived(ramp(progress, ...SCENE_SLIDE_RANGE));
  const leadIn = $derived(ramp(progress, ...SCENE_LEAD_RANGE));

  const revealed = '[clamp(0, calc((var(--fill) - var(--at)) / var(--edge)), 1)]';

  const rootClass = css({ display: 'grid', gridTemplateRows: '[auto auto]', alignContent: 'center', width: 'full', height: 'full' });
  const leadHostClass = css({
    display: { base: 'none', lg: 'block' },
    width: 'full',
    maxWidth: '[var(--track)]',
    marginX: 'auto',
    paddingBottom: '[var(--lead-gap)]',
  });
  const gridHostClass = css({
    width: 'full',
    maxWidth: '[var(--track)]',
    marginX: 'auto',
    '--shift': '[calc((var(--slide-progress) - 1) * var(--lead-height) / 2)]',
    transform: '[translateY(var(--shift))]',
    willChange: 'transform',
  });
  const monthGridClass = css({
    display: { base: 'grid', lg: 'none' },
    gridTemplateColumns: { base: '[repeat(3, minmax(0, 1fr))]', md: '[repeat(4, minmax(0, 1fr))]' },
    gap: { base: '12px', md: '16px' },
    width: 'full',
  });
  const monthBlockClass = css({ display: 'grid', gap: '4px' });
  const monthNameClass = css({
    fontSize: '10px',
    fontWeight: 'medium',
    color: 'text.hint',
    whiteSpace: 'nowrap',
    opacity: revealed,
  });
  const dayGridClass = css({ display: 'grid', gridTemplateColumns: '[repeat(7, minmax(0, 1fr))]', gap: '3px' });
  const gridClass = css({
    display: { base: 'none', lg: 'grid' },
    gridTemplateRows: '[auto repeat(7, minmax(0, 1fr))]',
    gridTemplateColumns: '[repeat(auto-fit, minmax(0px, 1fr))]',
    gridAutoFlow: 'column',
    gridAutoColumns: '[minmax(0, 1fr)]',
    gap: { base: '2px', md: '3px' },
    width: 'full',
  });
  const monthClass = css({
    gridRow: '1',
    paddingY: '[1px]',
    fontSize: '10px',
    fontWeight: 'medium',
    color: 'text.hint',
    whiteSpace: 'nowrap',
    overflow: 'visible',
    opacity: revealed,
  });
  const cellClass = css({
    aspectRatio: '1/1',
    borderRadius: '2px',
    backgroundColor: 'surface.inset',
    opacity: revealed,
    '&[data-level="1"]': { backgroundColor: 'palette.green/30' },
    '&[data-level="2"]': { backgroundColor: 'palette.green/50' },
    '&[data-level="3"]': { backgroundColor: 'palette.green/70' },
    '&[data-level="4"]': { backgroundColor: 'palette.green/85' },
    '&[data-level="5"]': { backgroundColor: 'palette.green' },
  });
</script>

<div
  style:--lead-gap="{LEAD_GAP}px"
  style:--lead-height="{leadHeight}px"
  style:--slide-progress={slide}
  style:--track="{TRACK}px"
  class={rootClass}
>
  <div class={leadHostClass} bind:clientHeight={leadHeight}>
    <SceneLead {cards} {cardsProgress} {lead} shown={leadIn} split />
  </div>

  <div class={gridHostClass}>
    <div style:--edge={EDGE} style:--fill={fill} class={monthGridClass} aria-hidden="true">
      {#each MONTH_BLOCKS as block, index (index)}
        <div class={monthBlockClass}>
          <div style:--at={block.at} class={monthNameClass}>{block.label}</div>
          <div class={dayGridClass}>
            {#each block.lead as lead (lead)}
              <div></div>
            {/each}
            {#each block.cells as cell, day (day)}
              <div style:--at={cell.at} class={cellClass} data-level={cell.level}></div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <div style:--edge={EDGE} style:--fill={fill} class={gridClass} aria-hidden="true">
      {#each MONTH_SPANS as span, index (index)}
        {#if span.end - span.start >= 1 || index === MONTH_SPANS.length - 1}
          <div style:grid-column="{span.start + 1} / {span.end + 2}" style:--at={span.at} class={monthClass}>
            {span.month}월
          </div>
        {/if}
      {/each}

      {#each CELLS as cell, index (index)}
        <div style:grid-row={cell.day + 2} style:--at={index} class={cellClass} data-level={cell.level}></div>
      {/each}
    </div>
  </div>
</div>
