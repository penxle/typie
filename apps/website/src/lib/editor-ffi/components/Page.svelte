<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { CROP_MARKER_SIZE } from '../constants';
  import { roundToScale } from '../geometry';
  import ExternalElement from './ExternalElement.svelte';
  import LinkOverlay from './LinkOverlay.svelte';
  import TableOverlay from './TableOverlay.svelte';
  import type { Editor } from '../editor.svelte';
  import type { EditorSurfaceHost } from '../editor-surface-host.svelte';

  type Props = {
    editor: Editor;
    surfaceHost: EditorSurfaceHost;
    page: number;
    width: number;
    height: number;
  };

  let { editor, surfaceHost, page, width, height }: Props = $props();

  const scaleFactor = $derived(editor.scaleFactor);
  const cssWidth = $derived(roundToScale(width, scaleFactor));
  const cssHeight = $derived(roundToScale(height, scaleFactor));
  const layoutMode = $derived(editor.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(editor.safeDisplayZoom());
  const slotWidth = $derived(roundToScale(width * displayZoom, scaleFactor));
  const slotHeight = $derived(roundToScale(height * displayZoom, scaleFactor));
  const showCropMarker = $derived(layoutMode?.type === 'paginated' && !editor.readOnly);
  const pagePresented = $derived(editor.published?.frames.has(page) === true);
  const externalElements = $derived.by(() => {
    void editor.publishedRevision;
    return pagePresented ? editor.pageExternalElements(page).filter((element) => element.data.type !== 'embed') : [];
  });
  const tableOverlays = $derived.by(() => {
    void editor.publishedRevision;
    return isPaginated && pagePresented ? editor.pageTableOverlays(page) : [];
  });
  const linkRects = $derived.by(() => {
    void editor.publishedRevision;
    return pagePresented ? editor.pageLinkRects(page) : [];
  });
</script>

<div style:width={`${slotWidth}px`} style:height={`${slotHeight}px`} class={css({ position: 'relative', flexShrink: '0' })}>
  <div
    style:width={`${cssWidth}px`}
    style:height={`${cssHeight}px`}
    style:transform={displayZoom === 1 ? undefined : `scale(${displayZoom})`}
    style:transform-origin={displayZoom === 1 ? undefined : 'top left'}
    style:will-change={displayZoom === 1 ? undefined : 'transform'}
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
      editor.pageEls[page] = el;
      return () => {
        editor.pageEls[page] = undefined;
      };
    }}
  >
    <div
      class={css({ position: 'absolute', inset: '0', overflow: 'hidden' })}
      {@attach (wrapper) => {
        const unregisterSurface = surfaceHost.registerPageContainer(page, wrapper);
        return () => unregisterSurface?.();
      }}
    ></div>

    {#each externalElements as element (element.node)}
      <ExternalElement {element} />
    {/each}

    {#each tableOverlays as overlay (`${overlay.table_id}-${overlay.page_idx}-${overlay.rows[0]?.index ?? 0}`)}
      <TableOverlay {overlay} readOnly={editor.readOnly} />
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
