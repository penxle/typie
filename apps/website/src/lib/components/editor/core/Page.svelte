<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import { CROP_MARKER_SIZE } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import ExternalArchived from '../external/ExternalArchived.svelte';
  import ExternalEmbed from '../external/ExternalEmbed.svelte';
  import ExternalFile from '../external/ExternalFile.svelte';
  import ExternalImage from '../external/ExternalImage.svelte';

  type Props = {
    page: number;
    containerEl?: HTMLDivElement;
  };

  let { page, containerEl = $bindable() }: Props = $props();

  const { editor } = getEditorContext();

  const pageWidth = $derived(editor.layout?.pages[page]?.width ?? 0);
  const pageHeight = $derived(editor.layout?.pages[page]?.height ?? 0);
  const marginTop = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginTop : 0);
  const marginBottom = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginBottom : 0);
  const marginLeft = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginLeft : 0);
  const marginRight = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.layout.layoutMode.pageMarginRight : 0);
  const layoutMode = $derived(editor.layout?.layoutMode);
  const externalElements = $derived(editor.externalElements.filter((el) => el.pageIdx === page));
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(isPaginated ? editor.displayZoom : 1);
  const slotWidth = $derived(pageWidth * displayZoom);
  const slotHeight = $derived(pageHeight * displayZoom);
  const remarkHighlightTarget = $derived(editor.remarkHighlightTarget?.pageIdx === page ? editor.remarkHighlightTarget : null);

  // NOTE: iOS에서 캔버스 롱프레스 시 텍스트 인식해서 선택되는 동작을 막음
  const disableCanvasPointer = $derived(editor.readOnly); // TODO: 항상 disable 해도 안전한지 확인하기

  let ctx2d = $state<CanvasRenderingContext2D | null>(null);
  let visible = $state(false);

  function render() {
    if (!ctx2d) return false;
    return editor.renderPageToCanvas(page, ctx2d);
  }

  $effect(() => {
    const renderVersion = editor.renderVersion;
    if (!visible || !ctx2d) return;
    untrack(() => {
      const ctx = editor.span?.spanContext();

      if (ctx) {
        editor.setTracing(ctx.traceId, ctx.spanId);
      }

      const rendered = render();

      if (ctx) {
        editor.clearTracing();
      }

      if (rendered) {
        editor.notifyPageRendered(page, renderVersion);
      }
    });
  });

  $effect(() => {
    const viewport = editor.scrollViewport;
    const node = containerEl;

    if (!node || !viewport) return;

    let rafId: number | null = null;

    const checkVisibility = () => {
      const rect = viewport.getRect();
      const viewportHeight = rect.bottom - rect.top;

      const pageRect = node.getBoundingClientRect();
      const marginPx = viewportHeight * 2;
      const isIntersecting = pageRect.bottom > rect.top - marginPx && pageRect.top < rect.bottom + marginPx;

      if (isIntersecting !== visible) {
        visible = isIntersecting;
      }

      if (isIntersecting) {
        const top = Math.max(rect.top, pageRect.top);
        const bottom = Math.min(rect.bottom, pageRect.bottom);
        const ratio = pageRect.height > 0 ? Math.max(0, bottom - top) / pageRect.height : 0;
        editor.updatePageVisibility(page, ratio);
      } else {
        editor.updatePageVisibility(page, 0);
      }
    };

    const scheduleCheck = () => {
      if (rafId === null) {
        rafId = requestAnimationFrame(() => {
          rafId = null;
          checkVisibility();
        });
      }
    };

    viewport.target.addEventListener('scroll', scheduleCheck);
    checkVisibility();

    return () => {
      viewport.target.removeEventListener('scroll', scheduleCheck);
      if (rafId !== null) {
        cancelAnimationFrame(rafId);
      }
      editor.updatePageVisibility(page, 0);
    };
  });
</script>

<div style:width={`${slotWidth}px`} style:height={`${slotHeight}px`} class={css({ position: 'relative', flexShrink: '0' })}>
  <div
    bind:this={containerEl}
    style:width={`${pageWidth}px`}
    style:height={`${pageHeight}px`}
    style:transform={isPaginated && displayZoom !== 1 ? `scale(${displayZoom})` : undefined}
    style:transform-origin={isPaginated && displayZoom !== 1 ? 'top left' : undefined}
    style:will-change={isPaginated && displayZoom !== 1 ? 'transform' : undefined}
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
    data-page-index={page}
  >
    {#if visible}
      <canvas
        style="image-rendering: pixelated;"
        style:pointer-events={disableCanvasPointer ? 'none' : 'auto'}
        class={css({ height: 'full', width: 'full' })}
        {@attach (canvas) => {
          ctx2d = canvas.getContext('2d');
          return () => {
            ctx2d = null;
          };
        }}
      ></canvas>

      {#each externalElements as el (el.nodeId)}
        {#if el.data.type === 'image'}
          <ExternalImage {el} />
        {:else if el.data.type === 'file'}
          <ExternalFile {el} />
        {:else if el.data.type === 'embed'}
          <ExternalEmbed {el} />
        {:else if el.data.type === 'archived'}
          <ExternalArchived {el} />
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

      {#each editor.trackedItems.filter((v) => v.group === 0 && v.pageIdx === page) as item (item.id)}
        {#each item.bounds as bound, i (i)}
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
            data-spellcheck-overlay={item.id}
          ></div>
        {/each}
      {/each}

      {#each editor.trackedItems.filter((v) => v.group === 1 && v.pageIdx === page) as item (item.id)}
        {#if editor.aiFeedbacks.find((v) => v.id === item.id)?.active}
          {#each item.bounds as bound, i (i)}
            <div
              style:left={`${bound.x}px`}
              style:top={`${bound.y}px`}
              style:width={`${bound.width}px`}
              style:height={`${bound.height}px`}
              class={css({
                position: 'absolute',
                pointerEvents: 'none',
                backgroundColor: { base: 'accent.brand.subtle', _dark: '[#544a2e]' },
                borderRadius: '2px',
                mixBlendMode: { base: 'multiply', _dark: 'screen' },
              })}
            ></div>
          {/each}
        {/if}
      {/each}

      {#each editor.trackedItems.filter((v) => v.group === 2 && v.pageIdx === page) as item (item)}
        {#each item.bounds as bound, i (i)}
          <div
            style:left={`${bound.x}px`}
            style:top={`${bound.y}px`}
            style:width={`${bound.width}px`}
            style:height={`${bound.height}px`}
            class={css({
              position: 'absolute',
              pointerEvents: 'none',
              borderRadius: '2px',
              mixBlendMode: { base: 'multiply', _dark: 'screen' },
              backgroundColor: editor.searchMatches.find((v) => v.id === item.id)?.active
                ? { base: '[#ffd280]', _dark: '[#a06828]' }
                : { base: '[#ffff80]', _dark: '[#544a2e]' },
            })}
          ></div>
        {/each}
      {/each}

      {#if remarkHighlightTarget}
        <div
          style:left={`${remarkHighlightTarget.bounds.x - 4}px`}
          style:top={`${remarkHighlightTarget.bounds.y - 4}px`}
          style:width={`${remarkHighlightTarget.bounds.width + 8}px`}
          style:height={`${remarkHighlightTarget.bounds.height + 8}px`}
          class={css({
            position: 'absolute',
            pointerEvents: 'none',
            borderRadius: '4px',
            backgroundColor: 'accent.brand.subtle',
            mixBlendMode: { base: 'multiply', _dark: 'screen' },
          })}
          transition:fade={{ duration: 150 }}
        ></div>
      {/if}

      {#if isPaginated && !editor.isReadOnly()}
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
