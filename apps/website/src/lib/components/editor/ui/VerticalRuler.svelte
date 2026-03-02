<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { calculateTicks, formatTickLabel } from '$lib/editor/ruler';

  type Props = {
    thickness?: number;
    pages: { width: number; height: number }[];
    pageGap: number;
    marginTop: number;
    marginBottom: number;
    zoom?: number;
    offsetY?: number;
    unit?: 'px' | 'cm';
    dpi?: number;
    padding?: number;
  };

  let {
    thickness = 24,
    pages,
    pageGap,
    marginTop,
    marginBottom,
    zoom = 1,
    offsetY = 0,
    unit = 'px',
    dpi = 96,
    padding = 0,
  }: Props = $props();

  const getTicksForPage = (height: number) => calculateTicks({ totalSize: height, unit, dpi, zoom });

  const isInMargin = (position: number, pageHeight: number): boolean => {
    return position < marginTop || position > pageHeight - marginBottom;
  };
</script>

<div
  style:transform={offsetY === 0 ? undefined : `translateY(-${offsetY}px)`}
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
  {#each pages as page, i (i)}
    {@const ticks = getTicksForPage(page.height)}
    <div
      style:height="{page.height * zoom}px"
      style:margin-bottom="{i === pages.length - 1 ? 0 : pageGap * zoom}px"
      class={css({ position: 'relative' })}
    >
      {#each ticks as tick (`${tick.logicalPosition}-${tick.isMajor ? 'm' : 's'}`)}
        {@const inMargin = isInMargin(tick.logicalPosition, page.height)}
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
              {formatTickLabel(tick.label, unit)}
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
