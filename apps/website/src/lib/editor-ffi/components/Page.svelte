<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { getEditorContext } from '../editor.svelte';
  import ExternalElement from './ExternalElement.svelte';
  import LinkOverlay from './LinkOverlay.svelte';
  import TableOverlay from './TableOverlay.svelte';

  type Props = {
    page: number;
    width: number;
    height: number;
  };

  let { page, width, height }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const cssWidth = $derived(Math.round(width * scaleFactor) / scaleFactor);
  const cssHeight = $derived(Math.round(height * scaleFactor) / scaleFactor);
  const isPaginated = $derived(ctx.editor?.rootAttrs?.layout_mode.type === 'paginated');
  const externalElements = $derived(ctx.editor?.externalElements.filter((element) => element.page_idx === page) ?? []);
  const tableOverlays = $derived(ctx.editor?.tableOverlays.filter((overlay) => overlay.page_idx === page) ?? []);
</script>

<div
  style:width={`${cssWidth}px`}
  style:height={`${cssHeight}px`}
  class={css({
    position: 'relative',
    isolation: 'isolate',
    flexShrink: '0',
    ...(isPaginated && {
      backgroundColor: 'surface.default',
      boxShadow: '[0_2px_8px_rgba(0,0,0,0.1)]',
      ringWidth: '1px',
      ringColor: 'black/5',
    }),
  })}
  {@attach (el) => {
    if (editor) {
      editor.pageEls[page] = el;

      return () => {
        editor.pageEls[page] = undefined;
      };
    }
  }}
>
  <canvas
    class={css({ height: 'full', width: 'full', imageRendering: 'pixelated' })}
    {@attach (canvas) => {
      if (!editor) return;

      untrack(() => {
        editor.attachSurface(page, canvas, width, height);
      });

      const off = editor.on('render_invalidated', () => {
        editor.renderSurface(page);
      });

      $effect.pre(() => {
        editor.resizeSurface(page, width, height);
        editor.renderSurface(page);
      });

      return () => {
        off();
        untrack(() => editor.detachSurface(page));
      };
    }}
  ></canvas>

  {#each externalElements as element (element.node_id)}
    <ExternalElement {element} />
  {/each}

  {#each tableOverlays as overlay (overlay.table_id)}
    <TableOverlay {overlay} />
  {/each}

  <LinkOverlay {page} />
</div>
