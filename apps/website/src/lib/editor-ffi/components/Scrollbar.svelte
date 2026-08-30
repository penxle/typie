<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pointerCapture } from '@typie/ui/actions';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { scrollElementFromWheel } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import { getEditorContext } from '../editor.svelte';
  import type { Size } from '@typie/editor-ffi/browser';

  const HIDE_DELAY = 1000;
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
  type ScrollDragSession = {
    axis: 'x' | 'y';
    startPointer: number;
    startScroll: number;
    maxScroll: number;
    trackMinusThumb: number;
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

  function sync() {
    if (!scrollContainer) return;
    metrics = {
      scrollTop: scrollContainer.scrollTop,
      scrollLeft: scrollContainer.scrollLeft,
      scrollHeight: scrollContainer.scrollHeight,
      scrollWidth: scrollContainer.scrollWidth,
      clientHeight: scrollContainer.clientHeight,
      clientWidth: scrollContainer.clientWidth,
    };
    containerRect = scrollContainer.getBoundingClientRect();
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

    const resizeObserver = new ResizeObserver(sync);
    resizeObserver.observe(el);

    return () => {
      resizeObserver.disconnect();
      clearTimeout(hideTimer);
    };
  });

  $effect(() => {
    const scroll = ctx.scroll;
    const revision = scroll?.lastScrollRevision ?? 0;
    const isAuto = scroll?.lastScrollWasAuto ?? true;
    if (revision === 0) return;

    sync();
    untrack(() => show(isAuto ? 'auto' : 'user'));
  });

  $effect(() => {
    void editor?.presentationGeometryRevision;
    sync();
  });

  function startDrag(axis: 'x' | 'y', e: PointerEvent): ScrollDragSession | null {
    if (!scrollContainer || dragAxis !== null || !e.isPrimary || e.button !== 0) return null;
    e.preventDefault();
    e.stopPropagation();
    ctx.scroll?.cancel();

    dragAxis = axis;
    show('user');

    const geometryetry = axis === 'y' ? y : x;
    return {
      axis,
      startPointer: axis === 'y' ? e.clientY : e.clientX,
      startScroll: axis === 'y' ? scrollContainer.scrollTop : scrollContainer.scrollLeft,
      maxScroll: geometryetry.maxScroll,
      trackMinusThumb: geometryetry.trackSize - geometryetry.thumbSize,
    };
  }

  function moveDrag(session: ScrollDragSession, e: PointerEvent) {
    if (!scrollContainer || dragAxis !== session.axis) return;
    const position = session.axis === 'y' ? e.clientY : e.clientX;
    const delta = ((position - session.startPointer) / session.trackMinusThumb) * session.maxScroll;
    if (session.axis === 'y') scrollContainer.scrollTop = session.startScroll + delta;
    else scrollContainer.scrollLeft = session.startScroll + delta;
  }

  function finishDrag(session: ScrollDragSession) {
    if (dragAxis !== session.axis) return;
    dragAxis = null;
    show('user');
  }

  function endDrag(session: ScrollDragSession, event: PointerEvent) {
    moveDrag(session, event);
    finishDrag(session);
  }

  function jumpTo(axis: 'x' | 'y', e: PointerEvent) {
    if (!scrollContainer || e.target !== e.currentTarget || !e.isPrimary || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    ctx.scroll?.cancel();
    show('user');

    const track = e.currentTarget as HTMLElement;
    const rect = track.getBoundingClientRect();
    const geometryetry = axis === 'y' ? y : x;

    const click = (axis === 'y' ? e.clientY - rect.top : e.clientX - rect.left) - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (click - geometryetry.thumbSize / 2) / (geometryetry.trackSize - geometryetry.thumbSize)));
    if (axis === 'y') scrollContainer.scrollTop = ratio * geometryetry.maxScroll;
    else scrollContainer.scrollLeft = ratio * geometryetry.maxScroll;
  }

  function handleWheel(event: WheelEvent) {
    ctx.scroll?.cancel();
    if (!scrollContainer) return;
    scrollElementFromWheel(scrollContainer, event);
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
    style:transition-duration={prefersReducedMotion.current ? '0ms' : undefined}
    class={css({
      pointerEvents: 'auto',
      position: 'fixed',
      zIndex: '10',
      transition: 'opacity',
      transitionDuration: '300ms',
      _motionReduce: { transitionDuration: '0ms' },
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
    onwheel={handleWheel}
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
      style:transition-duration={prefersReducedMotion.current ? '0ms' : undefined}
      class={css({
        position: 'absolute',
        cursor: 'pointer',
        borderRadius: 'full',
        transition: 'colors',
        backgroundColor: isDraggingThis
          ? isUserMode
            ? 'surface.inverse/80'
            : 'surface.inverse/45'
          : isUserMode
            ? 'surface.inverse/50'
            : 'surface.inverse/22',
        _hover: { backgroundColor: 'surface.inverse/80' },
        _active: { backgroundColor: 'surface.inverse/80' },
        _motionReduce: { transition: '[none]' },
      })}
      aria-valuemax={geometry.maxScroll}
      aria-valuemin={0}
      aria-valuenow={isVertical ? metrics.scrollTop : metrics.scrollLeft}
      role="slider"
      tabindex="-1"
      use:pointerCapture={{
        start: (event) => startDrag(axis, event),
        move: moveDrag,
        end: endDrag,
        cancel: finishDrag,
      }}
    ></div>
  </div>
{/snippet}

{#if y.canScroll && containerRect}
  <div
    style:top="{indicatorTop}px"
    style:right="{indicatorRight}px"
    style:transition-duration={prefersReducedMotion.current ? '0ms' : undefined}
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
      _motionReduce: { transitionDuration: '0ms' },
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
