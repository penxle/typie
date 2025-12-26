<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { calculateTicks, formatTickLabel } from '$lib/editor/ruler';

  type Props = {
    thickness?: number;
    pageWidth: number;
    marginLeft: number;
    marginRight: number;
    unit?: 'px' | 'cm';
    dpi?: number;
    padding?: number;
    ref?: HTMLElement | null;
  };

  let { thickness = 24, pageWidth, marginLeft, marginRight, unit = 'px', dpi = 96, padding = 0, ref = $bindable(null) }: Props = $props();

  const ticks = $derived(calculateTicks(pageWidth, unit, dpi));

  const isInMargin = (position: number): boolean => {
    return position < marginLeft || position > pageWidth - marginRight;
  };
</script>

<div
  style:height="{thickness}px"
  style:padding-left="{padding}px"
  style:padding-right="{padding}px"
  class={css({
    position: 'relative',
    display: 'flex',
    justifyContent: 'center',
    borderBottomWidth: '1px',
    borderColor: 'border.strong',
    backgroundColor: 'surface.default',
    userSelect: 'none',
  })}
>
  <div bind:this={ref} style:width="{pageWidth}px" class={css({ position: 'relative' })}>
    {#each ticks as tick (tick.position)}
      {@const inMargin = isInMargin(tick.position)}
      {#if tick.isMajor}
        <div
          style:left="{tick.position}px"
          style:height="8px"
          class={css({
            position: 'absolute',
            bottom: '0',
            width: '1px',
            backgroundColor: inMargin ? 'text.disabled' : 'text.muted',
          })}
        ></div>
        {#if tick.label}
          <div
            style:left="{tick.position}px"
            class={css({
              position: 'absolute',
              top: '2px',
              transform: 'translateX(-50%)',
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
          style:left="{tick.position}px"
          style:height="4px"
          class={css({
            position: 'absolute',
            bottom: '0',
            width: '1px',
            backgroundColor: inMargin ? 'border.strong' : 'text.faint',
          })}
        ></div>
      {/if}
    {/each}
  </div>
</div>
