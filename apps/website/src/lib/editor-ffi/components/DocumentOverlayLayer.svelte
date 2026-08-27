<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import { resolvePageSpans, roundToScale } from '../geometry';
  import TableOverlay from './TableOverlay.svelte';

  type PageAnchor = {
    top: number;
    slotWidth: number;
    slotHeight: number;
    logicalWidth: number;
    logicalHeight: number;
  };

  const ctx = getEditorContext();
  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const displayZoom = $derived(ctx.editor?.safeDisplayZoom() ?? 1);
  const pageAnchors = $derived.by(() => {
    const pageSizes = ctx.editor?.pageSizes ?? [];
    return resolvePageSpans(pageSizes, { scaleFactor, displayZoom }).map<PageAnchor>(({ page, top, bottom }) => ({
      top,
      slotWidth: roundToScale(pageSizes[page].width * displayZoom, scaleFactor),
      slotHeight: bottom - top,
      logicalWidth: roundToScale(pageSizes[page].width, scaleFactor),
      logicalHeight: roundToScale(pageSizes[page].height, scaleFactor),
    }));
  });
  const tableOverlays = $derived.by(() => {
    const editor = ctx.editor;
    const frames = editor?.published?.frames;
    return frames ? editor.tableOverlays.filter((overlay) => frames.has(overlay.page_idx)) : [];
  });
</script>

{#if ctx.editor?.rootAttrs?.layout_mode.type === 'continuous'}
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
    })}
  >
    {#each tableOverlays as overlay (overlay.table_id)}
      {@const anchor = pageAnchors[overlay.page_idx]}
      {#if anchor}
        <div
          style:top={`${anchor.top}px`}
          style:width={`${anchor.slotWidth}px`}
          style:height={`${anchor.slotHeight}px`}
          class={css({ position: 'absolute', left: '0', right: '0', marginX: 'auto', overflow: 'visible', pointerEvents: 'none' })}
        >
          <div
            style:width={`${anchor.logicalWidth}px`}
            style:height={`${anchor.logicalHeight}px`}
            style:transform={displayZoom === 1 ? undefined : `scale(${displayZoom})`}
            style:transform-origin={displayZoom === 1 ? undefined : 'top left'}
            class={css({ position: 'relative' })}
          >
            <TableOverlay {overlay} readOnly={ctx.editor.readOnly} />
          </div>
        </div>
      {/if}
    {/each}
  </div>
{/if}
