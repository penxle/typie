<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import { resolvePageSpans, roundToScale } from '../geometry';
  import TableOverlay from './TableOverlay.svelte';

  type PageAnchor = {
    top: number;
    width: number;
    height: number;
  };

  const ctx = getEditorContext();
  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const pageAnchors = $derived.by(() => {
    const pageSizes = ctx.editor?.pageSizes ?? [];
    return resolvePageSpans(pageSizes, { scaleFactor }).map<PageAnchor>(({ page, top, bottom }) => ({
      top,
      width: roundToScale(pageSizes[page].width, scaleFactor),
      height: bottom - top,
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
          style:width={`${anchor.width}px`}
          style:height={`${anchor.height}px`}
          class={css({ position: 'absolute', left: '0', right: '0', marginX: 'auto', overflow: 'visible', pointerEvents: 'none' })}
        >
          <TableOverlay {overlay} readOnly={ctx.editor.readOnly} />
        </div>
      {/if}
    {/each}
  </div>
{/if}
