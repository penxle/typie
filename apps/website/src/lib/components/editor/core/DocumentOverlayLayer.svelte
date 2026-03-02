<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SvelteMap } from 'svelte/reactivity';
  import { PAGE_GAP } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import TableOverlay from './TableOverlay.svelte';

  const { editor } = getEditorContext();

  const tableOverlaysByPage = $derived.by(() => {
    const grouped = new SvelteMap<number, typeof editor.tableOverlays>();
    for (const overlay of editor.tableOverlays) {
      const current = grouped.get(overlay.pageIdx);
      if (current) {
        current.push(overlay);
      } else {
        grouped.set(overlay.pageIdx, [overlay]);
      }
    }
    return grouped;
  });

  const displayZoom = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.displayZoom : 1);
  const pageGap = $derived(editor.layout?.layoutMode.type === 'paginated' ? PAGE_GAP * displayZoom : 0);
</script>

{#if !editor.readOnly && !editor.containerResizing}
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
    })}
  >
    <div
      style:gap={`${pageGap}px`}
      class={css({
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        width: 'full',
        height: 'full',
      })}
    >
      {#each editor.layout?.pages ?? [] as page, pageIdx (`page-${pageIdx}`)}
        {@const overlays = tableOverlaysByPage.get(pageIdx) ?? []}
        <div
          style:width={`${page.width * displayZoom}px`}
          style:height={`${page.height * displayZoom}px`}
          class={css({
            position: 'relative',
            pointerEvents: 'none',
          })}
        >
          <div
            style:width={`${page.width}px`}
            style:height={`${page.height}px`}
            style:transform={displayZoom === 1 ? undefined : `scale(${displayZoom})`}
            style:transform-origin={displayZoom === 1 ? undefined : 'top left'}
            class={css({
              position: 'relative',
              pointerEvents: 'none',
            })}
            data-page-index={pageIdx}
          >
            {#each overlays as overlay (`${overlay.tableId}-${overlay.startRowIndex}`)}
              <TableOverlay {editor} {overlay} />
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
