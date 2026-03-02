<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { calculateTicks, formatTickLabel } from '$lib/editor/ruler';

  type Props = {
    thickness?: number;
    pageWidth: number;
    marginLeft: number;
    marginRight: number;
    zoom?: number;
    offsetX?: number;
    unit?: 'px' | 'cm';
    dpi?: number;
    padding?: number;
  };

  let { thickness = 24, pageWidth, marginLeft, marginRight, zoom = 1, offsetX = 0, unit = 'px', dpi = 96, padding = 0 }: Props = $props();

  const visualPageWidth = $derived(pageWidth * zoom);
  const ticks = $derived(calculateTicks({ totalSize: pageWidth, unit, dpi, zoom }));

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
  <div
    style:transform={offsetX === 0 ? undefined : `translateX(-${offsetX}px)`}
    style:width="{visualPageWidth}px"
    class={css({ position: 'relative' })}
  >
    {#each ticks as tick (`${tick.logicalPosition}-${tick.isMajor ? 'm' : 's'}`)}
      {@const inMargin = isInMargin(tick.logicalPosition)}
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
            {formatTickLabel(tick.label, unit)}
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
