<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { CROP_MARKER_SIZE } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';
  import { WebGLRenderer } from '$lib/editor/webgl';

  type Props = {
    page: number;
    containerEl?: HTMLDivElement;
  };

  let { page, containerEl = $bindable() }: Props = $props();

  const editor = getEditor();

  const pageWidth = $derived(editor.layout.pageWidth);
  const pageHeight = $derived(editor.layout.pageHeights[page] ?? 0);
  const margin = $derived(editor.layout.layoutMode.type === 'paginated' ? editor.layout.pageMargin : 0);
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

<div class={css({ position: 'relative' })}>
  <div
    bind:this={containerEl}
    style:width={`${pageWidth}px`}
    style:height={`${pageHeight}px`}
    class={css({
      position: 'relative',
      backgroundColor: 'surface.default',
      ...(isPaginated && { boxShadow: '[0_2px_8px_rgba(0,0,0,0.1)]', ringWidth: '1px', ringColor: 'black/5' }),
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
          <div
            style:left="{el.bounds.x}px"
            style:top="{el.bounds.y}px"
            style:width="{el.bounds.width}px"
            style:height="{el.bounds.height}px"
            class={css({ pointerEvents: 'none', position: 'absolute', userSelect: 'none' })}
            data-node-id={el.nodeId}
          >
            <img class={css({ height: 'full', width: 'full' })} alt="" src={el.data.src} />
            {#if el.isSelected}
              <div style="background-color: rgba(153, 204, 255, 0.3);" class={css({ position: 'absolute', inset: '0' })}></div>
            {/if}
          </div>
        {/if}
      {/each}

      {#if isPaginated}
        <svg
          class={css({ pointerEvents: 'none', position: 'absolute', inset: '0', height: 'full', width: 'full', overflow: 'visible' })}
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d={`M ${margin} ${margin - CROP_MARKER_SIZE} L ${margin} ${margin} L ${margin - CROP_MARKER_SIZE} ${margin} M ${pageWidth - margin} ${margin - CROP_MARKER_SIZE} L ${pageWidth - margin} ${margin} L ${pageWidth - margin + CROP_MARKER_SIZE} ${margin} M ${margin} ${pageHeight - margin + CROP_MARKER_SIZE} L ${margin} ${pageHeight - margin} L ${margin - CROP_MARKER_SIZE} ${pageHeight - margin} M ${pageWidth - margin} ${pageHeight - margin + CROP_MARKER_SIZE} L ${pageWidth - margin} ${pageHeight - margin} L ${pageWidth - margin + CROP_MARKER_SIZE} ${pageHeight - margin}`}
            fill="none"
            stroke="rgba(0,0,0,0.15)"
          />
        </svg>
      {/if}
    {/if}
  </div>
</div>
