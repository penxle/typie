<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tick } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import TableOverlay from './TableOverlay.svelte';

  const { editor } = getEditorContext();

  let layoutRefreshVersion = $state(0);

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
    return [...grouped.entries()];
  });

  $effect(() => {
    void editor.layout.layoutMode;

    let disposed = false;

    void (async () => {
      await tick();
      if (disposed) return;
      layoutRefreshVersion += 1;
    })();

    return () => {
      disposed = true;
    };
  });

  function pageOffset(pageIdx: number): { left: number; top: number } | null {
    const pageEl = editor.pageContainerEls[pageIdx];
    const containerEl = editor.extensionArea.containerEl;
    if (!pageEl || !containerEl) {
      return null;
    }
    const pageRect = pageEl.getBoundingClientRect();
    const containerRect = containerEl.getBoundingClientRect();
    return {
      left: pageRect.left - containerRect.left,
      top: pageRect.top - containerRect.top,
    };
  }
</script>

{#if !editor.readOnly}
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
      zIndex: '2',
    })}
  >
    {#key layoutRefreshVersion}
      {#each tableOverlaysByPage as [pageIdx, overlays] (`table-page-${pageIdx}`)}
        {@const page = editor.layout.pages[pageIdx]}
        {@const offset = pageOffset(pageIdx)}
        {#if page && offset}
          <div
            style:left={`${offset.left}px`}
            style:top={`${offset.top}px`}
            style:width={`${page.width}px`}
            style:height={`${page.height}px`}
            class={css({
              position: 'absolute',
              pointerEvents: 'none',
            })}
          >
            {#each overlays as overlay (`${overlay.tableId}-${overlay.startRowIndex}`)}
              <TableOverlay {editor} {overlay} />
            {/each}
          </div>
        {/if}
      {/each}
    {/key}
  </div>
{/if}
