<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { calculateTicks, formatTickLabel } from '$lib/editor/ruler';

  type Props = {
    thickness?: number;
    pageHeights: number[];
    pageGap: number;
    marginTop: number;
    marginBottom: number;
    unit?: 'px' | 'cm';
    dpi?: number;
    padding?: number;
    ref?: HTMLElement | null;
  };

  let {
    thickness = 24,
    pageHeights,
    pageGap,
    marginTop,
    marginBottom,
    unit = 'px',
    dpi = 96,
    padding = 0,
    ref = $bindable(null),
  }: Props = $props();

  const getTicksForPage = (height: number) => calculateTicks(height, unit, dpi);

  const isInMargin = (position: number, pageHeight: number): boolean => {
    return position < marginTop || position > pageHeight - marginBottom;
  };
</script>

<div
  bind:this={ref}
  style:width="{thickness}px"
  style:padding-top="{padding}px"
  style:padding-bottom="{padding}px"
  class={css({
    position: 'relative',
    borderRightWidth: '1px',
    borderColor: 'border.strong',
    backgroundColor: 'surface.default',
    userSelect: 'none',
  })}
>
  {#each pageHeights as pageHeight, i (i)}
    {@const ticks = getTicksForPage(pageHeight)}
    <div
      style:height="{pageHeight}px"
      style:margin-bottom="{i === pageHeights.length - 1 ? 0 : pageGap}px"
      class={css({ position: 'relative' })}
    >
      {#each ticks as tick (tick.position)}
        {@const inMargin = isInMargin(tick.position, pageHeight)}
        {#if tick.isMajor}
          <div
            style:top="{tick.position}px"
            style:width="8px"
            class={css({
              position: 'absolute',
              right: '0',
              height: '1px',
              backgroundColor: inMargin ? 'text.disabled' : 'text.muted',
            })}
          ></div>
          {#if tick.label}
            <div
              style:top="{tick.position}px"
              class={css({
                position: 'absolute',
                right: '2px',
                width: '20px',
                transform: 'translateY(-50%) rotate(-90deg)',
                textAlign: 'center',
                fontFamily: 'mono',
                fontSize: '9px',
                color: inMargin ? 'text.disabled' : 'text.muted',
              })}
            >
              {formatTickLabel(Number(tick.label), unit)}
            </div>
          {/if}
        {:else}
          <div
            style:top="{tick.position}px"
            style:width="4px"
            class={css({
              position: 'absolute',
              right: '0',
              height: '1px',
              backgroundColor: inMargin ? 'border.strong' : 'text.faint',
            })}
          ></div>
        {/if}
      {/each}
    </div>
  {/each}
</div>
