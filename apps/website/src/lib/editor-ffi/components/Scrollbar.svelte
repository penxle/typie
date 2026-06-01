<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import type { Size } from '@typie/editor-ffi/browser';

  const HIDE_DELAY = 1000;
  const USER_SCROLL_WINDOW_MS = 220;
  const MIN_THUMB_SIZE = 30;
  const TRACK_PADDING = 2;
  const INDICATOR_HEIGHT = 24;
  const INDICATOR_GAP = 14;

  const ctx = getEditorContext();
  const scrollContainer = $derived(ctx.editor?.scrollContainerEl);

  type AxisMetric = { scrollPos: number; contentSize: number; viewportSize: number };
  type AxisGeometry = {
    canScroll: boolean;
    maxScroll: number;
    ratio: number;
    trackSize: number;
    thumbSize: number;
    thumbPos: number;
  };

  function axisGeometry(m: AxisMetric): AxisGeometry {
    const canScroll = m.contentSize > m.viewportSize;
    const maxScroll = Math.max(0, m.contentSize - m.viewportSize);
    const ratio = maxScroll > 0 ? m.scrollPos / maxScroll : 0;
    const trackSize = m.viewportSize - TRACK_PADDING * 2;
    const thumbSize = Math.max(MIN_THUMB_SIZE, (m.viewportSize / m.contentSize) * trackSize);
    const thumbPos = TRACK_PADDING + ratio * (trackSize - thumbSize);
    return { canScroll, maxScroll, ratio, trackSize, thumbSize, thumbPos };
  }

  function mostVisiblePage(
    scrollTop: number,
    clientHeight: number,
    scrollContainer: HTMLElement,
    pageEls: Record<number, HTMLDivElement | undefined>,
    sizes: Size[],
    displayZoom: number,
  ): number {
    if (sizes.length === 0) return 0;
    const viewportTop = scrollTop;
    const viewportBottom = scrollTop + clientHeight;
    const containerRect = scrollContainer.getBoundingClientRect();

    const pageTopInScrollContent = (el: HTMLElement) => el.getBoundingClientRect().top - containerRect.top + scrollTop;

    let lo = 0;
    let hi = sizes.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      const el = pageEls[mid];
      if (!el) return 0;
      if (pageTopInScrollContent(el) + sizes[mid].height * displayZoom <= viewportTop) lo = mid + 1;
      else hi = mid;
    }

    let bestPage = lo;
    let bestRatio = -1;
    for (let i = lo; i < sizes.length; i++) {
      const el = pageEls[i];
      if (!el) break;
      const pageTop = pageTopInScrollContent(el);
      const pageHeight = sizes[i].height * displayZoom;
      const pageBottom = pageTop + pageHeight;
      if (pageTop >= viewportBottom) break;
      const inter = Math.max(0, Math.min(pageBottom, viewportBottom) - Math.max(pageTop, viewportTop));
      const ratio = inter / pageHeight;
      if (ratio > bestRatio) {
        bestRatio = ratio;
        bestPage = i;
      }
    }
    return bestPage;
  }

  let metrics = $state({
    scrollTop: 0,
    scrollLeft: 0,
    scrollHeight: 0,
    scrollWidth: 0,
    clientHeight: 0,
    clientWidth: 0,
  });
  let containerRect = $state<DOMRect | null>(null);

  let dragAxis = $state<'x' | 'y' | null>(null);
  let hoverAxis = $state<'x' | 'y' | null>(null);
  let userScrollActive = $state(false);
  let userScrollTimer: ReturnType<typeof setTimeout> | undefined;

  let mode = $state<'hidden' | 'user' | 'auto'>('hidden');
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const isVisible = $derived(mode !== 'hidden' || dragAxis !== null);
  const isUserMode = $derived(mode === 'user');

  const editor = $derived(ctx.editor);
  const isPaginated = $derived(editor?.rootAttrs?.layout_mode.type === 'paginated');

  const y = $derived(
    axisGeometry({
      scrollPos: metrics.scrollTop,
      contentSize: metrics.scrollHeight,
      viewportSize: metrics.clientHeight,
    }),
  );
  const x = $derived(
    axisGeometry({
      scrollPos: metrics.scrollLeft,
      contentSize: metrics.scrollWidth,
      viewportSize: metrics.clientWidth,
    }),
  );

  const indicatorTop = $derived(containerRect ? containerRect.top + y.thumbPos + y.thumbSize / 2 - INDICATOR_HEIGHT / 2 : 0);
  const indicatorRight = $derived(containerRect ? window.innerWidth - containerRect.right + INDICATOR_GAP : 0);

  const indicatorText = $derived.by(() => {
    if (!editor) return '';
    if (isPaginated) {
      if (editor.pageSizes.length === 0) return '';
      if (!scrollContainer) return '';
      const page = mostVisiblePage(
        metrics.scrollTop,
        metrics.clientHeight,
        scrollContainer,
        editor.pageEls,
        editor.pageSizes,
        editor.safeDisplayZoom(),
      );
      return `${page + 1}/${editor.pageSizes.length}`;
    }
    return `${Math.round(y.ratio * 100)}%`;
  });

  function markUserScroll() {
    userScrollActive = true;
    clearTimeout(userScrollTimer);
    userScrollTimer = setTimeout(() => (userScrollActive = false), USER_SCROLL_WINDOW_MS);
  }

  function show(next: 'user' | 'auto') {
    mode = next;
    clearTimeout(hideTimer);
    if (dragAxis === null && hoverAxis === null) {
      hideTimer = setTimeout(() => (mode = 'hidden'), HIDE_DELAY);
    }
  }

  $effect(() => {
    const el = scrollContainer;
    if (!el) return;

    const sync = () => {
      metrics = {
        scrollTop: el.scrollTop,
        scrollLeft: el.scrollLeft,
        scrollHeight: el.scrollHeight,
        scrollWidth: el.scrollWidth,
        clientHeight: el.clientHeight,
        clientWidth: el.clientWidth,
      };
      containerRect = el.getBoundingClientRect();
    };

    const handleScroll = () => {
      sync();
      show(userScrollActive || dragAxis !== null ? 'user' : 'auto');
    };

    const handleUserInput = () => {
      markUserScroll();
      show('user');
    };

    const resizeObserver = new ResizeObserver(sync);
    resizeObserver.observe(el);

    el.addEventListener('scroll', handleScroll);
    el.addEventListener('wheel', handleUserInput, { passive: true });
    el.addEventListener('touchmove', handleUserInput, { passive: true });

    sync();

    return () => {
      el.removeEventListener('scroll', handleScroll);
      el.removeEventListener('wheel', handleUserInput);
      el.removeEventListener('touchmove', handleUserInput);
      resizeObserver.disconnect();
      clearTimeout(hideTimer);
      clearTimeout(userScrollTimer);
    };
  });

  function startDrag(axis: 'x' | 'y', e: PointerEvent) {
    if (!scrollContainer) return;
    e.preventDefault();
    e.stopPropagation();

    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);

    dragAxis = axis;
    markUserScroll();
    show('user');

    const geometryetry = axis === 'y' ? y : x;
    const startPointer = axis === 'y' ? e.clientY : e.clientX;
    const startScroll = axis === 'y' ? scrollContainer.scrollTop : scrollContainer.scrollLeft;
    const maxScroll = geometryetry.maxScroll;
    const trackMinusThumb = geometryetry.trackSize - geometryetry.thumbSize;

    const onMove = (ev: PointerEvent) => {
      if (!scrollContainer) return;
      markUserScroll();
      const delta = (((axis === 'y' ? ev.clientY : ev.clientX) - startPointer) / trackMinusThumb) * maxScroll;
      if (axis === 'y') scrollContainer.scrollTop = startScroll + delta;
      else scrollContainer.scrollLeft = startScroll + delta;
    };

    const end = () => {
      dragAxis = null;
      target.removeEventListener('pointermove', onMove);
      target.removeEventListener('pointerup', end);
      target.removeEventListener('lostpointercapture', end);
      show('user');
    };

    target.addEventListener('pointermove', onMove);
    target.addEventListener('pointerup', end);
    target.addEventListener('lostpointercapture', end);
  }

  function jumpTo(axis: 'x' | 'y', e: PointerEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
    e.preventDefault();
    e.stopPropagation();
    markUserScroll();
    show('user');

    const track = e.currentTarget as HTMLElement;
    const rect = track.getBoundingClientRect();
    const geometryetry = axis === 'y' ? y : x;

    const click = axis === 'y' ? e.clientY - rect.top - TRACK_PADDING : e.clientX - rect.left - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (click - geometryetry.thumbSize / 2) / (geometryetry.trackSize - geometryetry.thumbSize)));
    if (axis === 'y') scrollContainer.scrollTop = ratio * geometryetry.maxScroll;
    else scrollContainer.scrollLeft = ratio * geometryetry.maxScroll;
  }
</script>

{#snippet track(axis: 'x' | 'y', geometry: AxisGeometry)}
  {@const isVertical = axis === 'y'}
  {@const isDraggingThis = dragAxis === axis}
  <div
    style:top={isVertical && containerRect ? `${containerRect.top}px` : undefined}
    style:right={isVertical && containerRect ? `${window.innerWidth - containerRect.right}px` : undefined}
    style:bottom={!isVertical && containerRect ? `${window.innerHeight - containerRect.bottom}px` : undefined}
    style:left={!isVertical && containerRect ? `${containerRect.left}px` : undefined}
    style:height={isVertical && containerRect ? `${containerRect.height - (x.canScroll ? 12 : 0)}px` : '12px'}
    style:width={!isVertical && containerRect ? `${containerRect.width - (y.canScroll ? 12 : 0)}px` : '12px'}
    class={css({
      pointerEvents: 'auto',
      position: 'fixed',
      zIndex: '10',
      transition: 'opacity',
      transitionDuration: '300ms',
      opacity: isVisible || isDraggingThis ? (isUserMode ? '100' : '65') : '0',
    })}
    aria-controls="scroll-content"
    aria-orientation={isVertical ? undefined : 'horizontal'}
    aria-valuemax={geometry.maxScroll}
    aria-valuemin={0}
    aria-valuenow={isVertical ? metrics.scrollTop : metrics.scrollLeft}
    onpointerdown={(e) => jumpTo(axis, e)}
    onpointerenter={() => {
      hoverAxis = axis;
      show('user');
    }}
    onpointerleave={() => {
      hoverAxis = null;
      show('user');
    }}
    role="scrollbar"
    tabindex="-1"
  >
    <div
      style:top={isVertical ? `${geometry.thumbPos}px` : undefined}
      style:right={isVertical ? '2px' : undefined}
      style:left={isVertical ? undefined : `${geometry.thumbPos}px`}
      style:bottom={isVertical ? undefined : '2px'}
      style:height={isVertical ? `${geometry.thumbSize}px` : '8px'}
      style:width={isVertical ? '8px' : `${geometry.thumbSize}px`}
      class={css({
        position: 'absolute',
        cursor: 'pointer',
        borderRadius: 'full',
        transition: 'colors',
        backgroundColor: isDraggingThis
          ? isUserMode
            ? 'surface.dark/80'
            : 'surface.dark/45'
          : isUserMode
            ? 'surface.dark/50'
            : 'surface.dark/22',
        _hover: { backgroundColor: 'surface.dark/80' },
        _active: { backgroundColor: 'surface.dark/80' },
      })}
      aria-valuemax={geometry.maxScroll}
      aria-valuemin={0}
      aria-valuenow={isVertical ? metrics.scrollTop : metrics.scrollLeft}
      onpointerdown={(e) => startDrag(axis, e)}
      role="slider"
      tabindex="-1"
    ></div>
  </div>
{/snippet}

{#if y.canScroll && containerRect}
  <div
    style:top="{indicatorTop}px"
    style:right="{indicatorRight}px"
    class={css({
      pointerEvents: 'none',
      position: 'fixed',
      zIndex: '20',
      borderRadius: '4px',
      backgroundColor: 'surface.dark/65',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '11px',
      whiteSpace: 'nowrap',
      color: 'text.bright',
      fontVariantNumeric: 'tabular-nums',
      transition: 'opacity',
      transitionDuration: '300ms',
      opacity: isVisible && isUserMode ? '100' : '0',
    })}
  >
    {indicatorText}
  </div>
{/if}

{#if y.canScroll && containerRect}
  {@render track('y', y)}
{/if}

{#if x.canScroll && containerRect}
  {@render track('x', x)}
{/if}
