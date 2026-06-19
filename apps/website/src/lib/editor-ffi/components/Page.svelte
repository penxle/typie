<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { CROP_MARKER_SIZE, PAGE_RENDER_OVERSCAN_MARGIN } from '../constants';
  import { ALL_LAYERS, getEditorContext, layersToMask } from '../editor.svelte';
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

  let canvases: (HTMLCanvasElement | undefined)[] = $state([undefined, undefined, undefined, undefined]);
  let pageEl = $state<HTMLElement>();
  let attached = $state(false);

  let isVisible = false;
  // Reactive mirror of `isVisible` used only by the overlay queries below, so
  // off-screen pages never build their fragments. Kept separate from the plain
  // `isVisible` so the imperative render effects are untouched.
  let overlaysVisible = $state(false);
  let dirtyLayers = 0;
  let needsResize = false;

  $effect(() => {
    if (!editor || !pageEl || canvases.some((c) => !c)) return;
    const cs = canvases as HTMLCanvasElement[];

    untrack(() => editor.attachSurface(page, cs, width, height));

    const paint = (layers: number) => {
      if (isVisible) editor.renderSurface(page, layers);
      else dirtyLayers |= layers;
    };
    const off = editor.on('render_invalidated', (_, ev) => paint(layersToMask(ev.layers)));

    if (isVisible) untrack(() => editor.renderSurface(page, ALL_LAYERS));
    else dirtyLayers = ALL_LAYERS;

    attached = true;

    return () => {
      attached = false;
      off();
      untrack(() => editor.detachSurface(page));
    };
  });

  $effect.pre(() => {
    if (!editor) return;
    void editor.surfaceScaleFactor;
    void width;
    void height;
    if (isVisible) {
      editor.resizeSurface(page, width, height);
      editor.renderSurface(page, ALL_LAYERS);
      needsResize = false;
      dirtyLayers = 0;
    } else {
      needsResize = true;
      dirtyLayers = ALL_LAYERS;
    }
  });

  $effect(() => {
    if (!editor || !attached || !pageEl) return;
    const root = editor.scrollRootEl;
    if (root === undefined) return;
    const observer = new IntersectionObserver(
      (entries) => {
        isVisible = entries.at(-1)?.isIntersecting ?? isVisible;
        overlaysVisible = isVisible;
        if (!isVisible) return;
        if (needsResize) {
          editor.resizeSurface(page, width, height);
          needsResize = false;
        }
        if (dirtyLayers) {
          editor.renderSurface(page, dirtyLayers);
          dirtyLayers = 0;
        }
      },
      { root, rootMargin: PAGE_RENDER_OVERSCAN_MARGIN, threshold: 0 },
    );
    observer.observe(pageEl);

    return () => observer.disconnect();
  });

  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const cssWidth = $derived(Math.round(width * scaleFactor) / scaleFactor);
  const cssHeight = $derived(Math.round(height * scaleFactor) / scaleFactor);
  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(isPaginated ? (ctx.editor?.displayZoom ?? 1) : 1);
  const slotWidth = $derived(Math.round(width * displayZoom * scaleFactor) / scaleFactor);
  const slotHeight = $derived(Math.round(height * displayZoom * scaleFactor) / scaleFactor);
  const showCropMarker = $derived(layoutMode?.type === 'paginated' && !(ctx.editor?.readOnly ?? false));
  // Per-visible-page queries: only on-screen pages build their fragment, turning
  // the old whole-document O(pages · N) recompute (every keystroke) into O(N) for
  // the few visible pages.
  const externalElements = $derived.by(() => {
    void ctx.editor?.tickRevision;
    return overlaysVisible && ctx.editor ? ctx.editor.pageExternalElements(page) : [];
  });
  const tableOverlays = $derived.by(() => {
    void ctx.editor?.tickRevision;
    return overlaysVisible && ctx.editor ? ctx.editor.pageTableOverlays(page) : [];
  });
  const linkRects = $derived.by(() => {
    void ctx.editor?.tickRevision;
    return overlaysVisible && ctx.editor ? ctx.editor.pageLinkRects(page) : [];
  });
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
        pageEl = el;

        return () => {
          editor.pageEls[page] = undefined;
          pageEl = undefined;
        };
      }
    }}
  >
    {#each [0, 1, 2, 3] as layer (layer)}
      <canvas
        style:z-index={layer}
        class={css({ position: 'absolute', inset: '0', height: 'full', width: 'full', imageRendering: 'pixelated' })}
        {@attach (canvas) => {
          canvases[layer] = canvas;
          return () => {
            canvases[layer] = undefined;
          };
        }}
      ></canvas>
    {/each}

    {#each externalElements as element (element.node_id)}
      <ExternalElement {element} />
    {/each}

    {#each tableOverlays as overlay (`${overlay.table_id}-${overlay.page_idx}-${overlay.rows[0]?.index ?? 0}`)}
      <TableOverlay {overlay} />
    {/each}

    <LinkOverlay links={linkRects} />

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
