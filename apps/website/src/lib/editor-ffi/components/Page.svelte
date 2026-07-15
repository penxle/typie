<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { CROP_MARKER_SIZE, PAGE_RENDER_OVERSCAN_MARGIN } from '../constants';
  import { getEditorContext } from '../editor.svelte';
  import { shouldKeepEmbedsWhileHidden, visibleExternalElements } from './external-element-visibility';
  import ExternalElement from './ExternalElement.svelte';
  import LinkOverlay from './LinkOverlay.svelte';
  import TableOverlay from './TableOverlay.svelte';

  type Props = {
    page: number;
    width: number;
    height: number;
    backingHeight: number;
  };

  let { page, width, height, backingHeight }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  // Reactive mirror of `isVisible` used only by the overlay queries below, so
  // off-screen pages never build their fragments. Kept separate from the plain
  // `isVisible` so the imperative render effects are untouched.
  let overlaysVisible = $state(false);
  // Embed iframes (e.g. a playing YouTube video) lose their state when unmounted,
  // so pages holding embeds keep them mounted while scrolled off-screen.
  let keepEmbedsWhileHidden = $state(false);

  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const cssWidth = $derived(Math.round(width * scaleFactor) / scaleFactor);
  const cssHeight = $derived(Math.round(height * scaleFactor) / scaleFactor);
  const cssBackingHeight = $derived(Math.round(backingHeight * scaleFactor) / scaleFactor);
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
    const editor = ctx.editor;
    return editor ? visibleExternalElements(overlaysVisible, keepEmbedsWhileHidden, () => editor.pageExternalElements(page)) : [];
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
      if (!editor) {
        return;
      }

      editor.pageEls[page] = el;

      return () => {
        editor.pageEls[page] = undefined;
      };
    }}
  >
    <div class={css({ position: 'absolute', inset: '0', overflow: 'hidden' })}>
      <canvas
        style:height={`${cssBackingHeight}px`}
        class={css({ position: 'absolute', top: '0', left: '0', width: 'full', imageRendering: 'pixelated' })}
        {@attach (canvas) => {
          if (!editor) return;

          let isVisible = false;
          let dirty = false;
          let needsResize = false;

          untrack(() => {
            editor.attachSurface(page, canvas, width, backingHeight);
          });

          const paint = () => {
            if (isVisible) {
              editor.requestSurfaceRender(page);
              dirty = false;
            } else {
              dirty = true;
            }
          };

          const off = editor.on('render_invalidated', paint);

          const onContextRestored = () => {
            editor.invalidateSurface(page);
            paint();
          };
          canvas.addEventListener('contextrestored', onContextRestored);

          $effect.pre(() => {
            void editor.surfaceScaleFactor;
            void width;
            void backingHeight;
            if (isVisible) {
              editor.requestSurfaceResize(page, width, backingHeight);
              dirty = false;
              needsResize = false;
            } else {
              needsResize = true;
              dirty = true;
            }
          });

          $effect(() => {
            const root = editor.scrollRootEl;
            if (root === undefined) return;

            const observer = new IntersectionObserver(
              (entries) => {
                isVisible = entries.at(-1)?.isIntersecting ?? isVisible;
                if (overlaysVisible && !isVisible) {
                  keepEmbedsWhileHidden = shouldKeepEmbedsWhileHidden(externalElements);
                }
                overlaysVisible = isVisible;
                if (!isVisible) return;
                if (needsResize) {
                  editor.requestSurfaceResize(page, width, backingHeight);
                  needsResize = false;
                  dirty = false;
                }
                if (dirty) {
                  editor.requestSurfaceRender(page);
                  dirty = false;
                }
              },
              { root, rootMargin: PAGE_RENDER_OVERSCAN_MARGIN, threshold: 0 },
            );
            observer.observe(canvas);

            return () => observer.disconnect();
          });

          return () => {
            canvas.removeEventListener('contextrestored', onContextRestored);
            off();
            untrack(() => editor.detachSurface(page));
          };
        }}
      ></canvas>
    </div>

    {#each externalElements as element (element.node)}
      <ExternalElement {element} />
    {/each}

    {#each tableOverlays as overlay (`${overlay.table_id}-${overlay.page_idx}-${overlay.rows[0]?.index ?? 0}`)}
      <TableOverlay {overlay} readOnly={ctx.editor?.readOnly ?? false} />
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
