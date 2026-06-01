<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { CROP_MARKER_SIZE } from '../constants';
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
  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(isPaginated ? (ctx.editor?.displayZoom ?? 1) : 1);
  const slotWidth = $derived(Math.round(width * displayZoom * scaleFactor) / scaleFactor);
  const slotHeight = $derived(Math.round(height * displayZoom * scaleFactor) / scaleFactor);
  const showCropMarker = $derived(layoutMode?.type === 'paginated' && !(ctx.editor?.readOnly ?? false));
  const externalElements = $derived(ctx.editor?.externalElements.filter((element) => element.page_idx === page) ?? []);
  const tableOverlays = $derived(ctx.editor?.tableOverlays.filter((overlay) => overlay.page_idx === page) ?? []);
</script>

<div style:width={`${slotWidth}px`} style:height={`${slotHeight}px`} class={css({ position: 'relative', flexShrink: '0' })}>
  <div
    style:width={`${cssWidth}px`}
    style:height={`${cssHeight}px`}
    style:transform={isPaginated && displayZoom !== 1 ? `scale(${displayZoom})` : undefined}
    style:transform-origin={isPaginated && displayZoom !== 1 ? 'top left' : undefined}
    style:will-change={isPaginated && displayZoom !== 1 ? 'transform' : undefined}
    class={css({
      position: 'relative',
      isolation: 'isolate',
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
          void editor.surfaceScaleFactor;
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

    {#if showCropMarker && layoutMode?.type === 'paginated'}
      {@const marginLeft = layoutMode.page_margin_left}
      {@const marginRight = layoutMode.page_margin_right}
      {@const marginTop = layoutMode.page_margin_top}
      {@const marginBottom = layoutMode.page_margin_bottom}
      <svg
        class={css({
          pointerEvents: 'none',
          position: 'absolute',
          inset: '0',
          height: 'full',
          width: 'full',
          overflow: 'visible',
          color: 'text.default',
          opacity: '15',
        })}
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d={`M ${marginLeft} ${marginTop - CROP_MARKER_SIZE} L ${marginLeft} ${marginTop} L ${marginLeft - CROP_MARKER_SIZE} ${marginTop} M ${width - marginRight} ${marginTop - CROP_MARKER_SIZE} L ${width - marginRight} ${marginTop} L ${width - marginRight + CROP_MARKER_SIZE} ${marginTop} M ${marginLeft} ${height - marginBottom + CROP_MARKER_SIZE} L ${marginLeft} ${height - marginBottom} L ${marginLeft - CROP_MARKER_SIZE} ${height - marginBottom} M ${width - marginRight} ${height - marginBottom + CROP_MARKER_SIZE} L ${width - marginRight} ${height - marginBottom} L ${width - marginRight + CROP_MARKER_SIZE} ${height - marginBottom}`}
          fill="none"
          stroke="currentColor"
        />
      </svg>
    {/if}
  </div>
</div>
