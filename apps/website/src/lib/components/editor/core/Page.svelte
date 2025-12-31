<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { CROP_MARKER_SIZE } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';
  import { WebGLRenderer } from '$lib/editor/webgl';
  import ExternalImage from './ExternalImage.svelte';

  type Props = {
    page: number;
    containerEl?: HTMLDivElement;
  };

  let { page, containerEl = $bindable() }: Props = $props();

  const editor = getEditor();

  const pageWidth = $derived(editor.layout.pageWidth);
  const pageHeight = $derived(editor.layout.pageHeights[page] ?? 0);
  const marginTop = $derived(editor.layout.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginTop : 0);
  const marginBottom = $derived(editor.layout.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginBottom : 0);
  const marginLeft = $derived(editor.layout.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginLeft : 0);
  const marginRight = $derived(editor.layout.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginRight : 0);
  const layoutMode = $derived(editor.layout.layoutMode);
  const mediaOnPage = $derived(editor.externalElements.filter((el) => el.pageIdx === page));
  const isPaginated = $derived(layoutMode.type === 'paginated');

  let renderer = $state<WebGLRenderer | null>(null);
  let visible = $state(false);

  function render() {
    if (!renderer) return;

    const info = editor.renderPage(page);
    if (!info) return;

    renderer.render(info.ptr, info.len, info.width, info.height);
  }

  $effect(() => {
    void editor.renderVersion;
    if (!visible || !renderer) return;
    render();
  });
</script>

<div class={css({ position: 'relative', maxWidth: 'full' })}>
  <div
    bind:this={containerEl}
    style:width={`${pageWidth}px`}
    style:height={`${pageHeight}px`}
    class={css({
      position: 'relative',
      ...(isPaginated && {
        backgroundColor: 'surface.default',
        boxShadow: '[0_2px_8px_rgba(0,0,0,0.1)]',
        ringWidth: '1px',
        ringColor: 'black/5',
      }),
      isolation: 'isolate',
    })}
    {@attach (node) => {
      const observer = new IntersectionObserver(
        ([entry]) => {
          visible = entry.isIntersecting;
          editor.updatePageVisibility(page, entry.intersectionRatio);
        },
        { rootMargin: '200% 0px', threshold: [0, 0.25, 0.5, 0.75, 1] },
      );
      observer.observe(node);
      return () => {
        observer.disconnect();
        editor.updatePageVisibility(page, 0);
      };
    }}
    data-page-index={page}
  >
    {#if visible}
      <canvas
        style="image-rendering: pixelated;"
        class={css({ height: 'full', width: 'full' })}
        {@attach (canvas) => {
          try {
            renderer = new WebGLRenderer(canvas);
          } catch (err) {
            console.error('WebGL init failed:', err);
          }

          return () => {
            renderer?.dispose();
            renderer = null;
          };
        }}
      ></canvas>

      {#each mediaOnPage as el (el.nodeId)}
        {#if el.data.type === 'image'}
          <ExternalImage {el} />
        {/if}
      {/each}

      {#if editor.readOnly}
        {#each editor.linkOverlays.filter((o) => o.pageIdx === page) as overlay, i (`${i}-${overlay.href}`)}
          {#each overlay.bounds as bound, j (`${j}-${overlay.href}`)}
            <a
              style:left={`${bound.x}px`}
              style:top={`${bound.y}px`}
              style:width={`${bound.width}px`}
              style:height={`${bound.height}px`}
              class={css({
                position: 'absolute',
                cursor: 'pointer',
                display: 'block',
              })}
              data-external-element
              href={overlay.href}
              rel="noopener noreferrer"
              target="_blank"
              title={overlay.href}
            ></a>
          {/each}
        {/each}
      {/if}

      {#each editor.spellcheckOverlays.filter((o) => o.pageIdx === page) as overlay, i (`${i}-${overlay.id}`)}
        {#each overlay.bounds as bound, j (`${j}-${overlay.id}`)}
          <div
            style:left={`${bound.x}px`}
            style:top={`${bound.y + bound.ascent + 2}px`}
            style:width={`${bound.width}px`}
            class={css({
              position: 'absolute',
              height: '4px',
              pointerEvents: 'none',
              backgroundImage: `url("data:image/svg+xml,${encodeURIComponent(
                '<svg width="6" height="3" viewBox="0 0 6 3" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M0 0.5C1.5 0.5 1.5 2.5 3 2.5C4.5 2.5 4.5 0.5 6 0.5" stroke="#DC2626" stroke-linecap="round"/></svg>',
              )}")`,
              backgroundRepeat: 'repeat-x',
              backgroundPosition: 'bottom',
            })}
            data-spellcheck-overlay={overlay.id}
          ></div>
        {/each}
      {/each}

      {#each editor.searchResults.overlays.filter((o) => o.pageIdx === page) as overlay, i (`search-${i}`)}
        {#each overlay.bounds as bound, j (`search-${i}-${j}`)}
          <div
            style:left={`${bound.x}px`}
            style:top={`${bound.y}px`}
            style:width={`${bound.width}px`}
            style:height={`${bound.height}px`}
            style:background-color={overlay.isCurrent ? 'rgba(255, 165, 0, 0.5)' : 'rgba(255, 255, 0, 0.5)'}
            class={css({
              position: 'absolute',
              pointerEvents: 'none',
              borderRadius: '2px',
              mixBlendMode: 'multiply',
            })}
          ></div>
        {/each}
      {/each}

      {#if isPaginated}
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
            d={`M ${marginLeft} ${marginTop - CROP_MARKER_SIZE} L ${marginLeft} ${marginTop} L ${marginLeft - CROP_MARKER_SIZE} ${marginTop} M ${pageWidth - marginRight} ${marginTop - CROP_MARKER_SIZE} L ${pageWidth - marginRight} ${marginTop} L ${pageWidth - marginRight + CROP_MARKER_SIZE} ${marginTop} M ${marginLeft} ${pageHeight - marginBottom + CROP_MARKER_SIZE} L ${marginLeft} ${pageHeight - marginBottom} L ${marginLeft - CROP_MARKER_SIZE} ${pageHeight - marginBottom} M ${pageWidth - marginRight} ${pageHeight - marginBottom + CROP_MARKER_SIZE} L ${pageWidth - marginRight} ${pageHeight - marginBottom} L ${pageWidth - marginRight + CROP_MARKER_SIZE} ${pageHeight - marginBottom}`}
            fill="none"
            stroke="currentColor"
          />
        </svg>
      {/if}
    {/if}
  </div>
</div>
