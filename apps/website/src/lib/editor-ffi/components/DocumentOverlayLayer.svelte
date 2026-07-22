<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import TableOverlay from './TableOverlay.svelte';

  type PageAnchor = {
    top: number;
    width: number;
    height: number;
  };

  const ctx = getEditorContext();
  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const pageAnchors = $derived.by(() => {
    const anchors: PageAnchor[] = [];
    let top = 0;
    for (const pageSize of ctx.editor?.pageSizes ?? []) {
      const width = roundToScale(pageSize.width, scaleFactor);
      const height = roundToScale(pageSize.height, scaleFactor);
      anchors.push({ top, width, height });
      top += height;
    }
    return anchors;
  });

  function roundToScale(value: number, scale: number): number {
    return Math.round(value * scale) / scale;
  }
</script>

{#if ctx.editor?.rootAttrs?.layout_mode.type === 'continuous'}
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
    })}
  >
    {#each ctx.editor.tableOverlays as overlay (overlay.table_id)}
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
