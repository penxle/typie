<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor/context.svelte';

  type Props = {
    scrollContainer: HTMLElement | null;
  };

  const HIDE_DELAY = 1000;
  const USER_SCROLL_WINDOW_MS = 220;
  const MIN_THUMB_SIZE = 30;
  const TRACK_PADDING = 2;
  const INDICATOR_HEIGHT = 24;
  const INDICATOR_GAP = 14;

  let { scrollContainer }: Props = $props();

  const { editor } = getEditorContext();

  let scrollTop = $state(0);
  let scrollHeight = $state(0);
  let clientHeight = $state(0);

  let scrollLeft = $state(0);
  let scrollWidth = $state(0);
  let clientWidth = $state(0);

  let containerRect = $state<DOMRect | null>(null);
  let isDraggingV = $state(false);
  let isDraggingH = $state(false);
  let isHoveringV = $state(false);
  let isHoveringH = $state(false);
  let isVisible = $state(false);
  let visibleScrollSource = $state<'user' | 'auto'>('user');
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let lastUserScrollInputAt = 0;
  let dragStartY = 0;
  let dragStartX = 0;
  let dragStartScrollTop = 0;
  let dragStartScrollLeft = 0;

  $effect(() => {
    if (!scrollContainer) return;

    const syncState = () => {
      scrollTop = scrollContainer.scrollTop;
      scrollHeight = scrollContainer.scrollHeight;
      clientHeight = scrollContainer.clientHeight;
      scrollLeft = scrollContainer.scrollLeft;
      scrollWidth = scrollContainer.scrollWidth;
      clientWidth = scrollContainer.clientWidth;
      containerRect = scrollContainer.getBoundingClientRect();
    };

    const isRecentUserScroll = () => Date.now() - lastUserScrollInputAt <= USER_SCROLL_WINDOW_MS;

    const handleScroll = () => {
      syncState();
      showTemporarily(isRecentUserScroll() || isDraggingV || isDraggingH ? 'user' : 'auto');
    };

    const handleUserScroll = () => {
      lastUserScrollInputAt = Date.now();
      showTemporarily('user');
    };

    const resizeObserver = new ResizeObserver(syncState);
    resizeObserver.observe(scrollContainer);

    scrollContainer.addEventListener('scroll', handleScroll);
    scrollContainer.addEventListener('wheel', handleUserScroll, { passive: true });
    scrollContainer.addEventListener('touchmove', handleUserScroll, { passive: true });
    syncState();

    return () => {
      scrollContainer.removeEventListener('scroll', handleScroll);
      scrollContainer.removeEventListener('wheel', handleUserScroll);
      scrollContainer.removeEventListener('touchmove', handleUserScroll);
      resizeObserver.disconnect();
      clearTimeout(hideTimer);
    };
  });

  function showTemporarily(source: 'user' | 'auto' = 'auto') {
    isVisible = true;
    visibleScrollSource = source;
    clearTimeout(hideTimer);
    if (!isDraggingV && !isDraggingH && !isHoveringV && !isHoveringH) {
      hideTimer = setTimeout(() => (isVisible = false), HIDE_DELAY);
    }
  }

  function handleHoverStartV() {
    isHoveringV = true;
    showTemporarily('user');
  }

  function handleHoverEndV() {
    isHoveringV = false;
    showTemporarily('user');
  }

  function handleHoverStartH() {
    isHoveringH = true;
    showTemporarily('user');
  }

  function handleHoverEndH() {
    isHoveringH = false;
    showTemporarily('user');
  }

  const canScrollV = $derived(scrollHeight > clientHeight);
  const maxScrollV = $derived(scrollHeight - clientHeight);
  const scrollRatioV = $derived(maxScrollV > 0 ? scrollTop / maxScrollV : 0);

  const trackHeight = $derived(clientHeight - TRACK_PADDING * 2);
  const thumbHeight = $derived(Math.max(MIN_THUMB_SIZE, (clientHeight / scrollHeight) * trackHeight));
  const thumbTop = $derived(TRACK_PADDING + scrollRatioV * (trackHeight - thumbHeight));

  const canScrollH = $derived(scrollWidth > clientWidth);
  const maxScrollH = $derived(scrollWidth - clientWidth);
  const scrollRatioH = $derived(maxScrollH > 0 ? scrollLeft / maxScrollH : 0);

  const trackWidth = $derived(clientWidth - TRACK_PADDING * 2);
  const thumbWidth = $derived(Math.max(MIN_THUMB_SIZE, (clientWidth / scrollWidth) * trackWidth));
  const thumbLeft = $derived(TRACK_PADDING + scrollRatioH * (trackWidth - thumbWidth));

  const indicatorTop = $derived(containerRect ? containerRect.top + thumbTop + thumbHeight / 2 - INDICATOR_HEIGHT / 2 : 0);
  const indicatorRight = $derived(containerRect ? window.innerWidth - containerRect.right + INDICATOR_GAP : 0);
  const isUserScrollVisible = $derived(visibleScrollSource === 'user');

  const displayText = $derived.by(() => {
    if (editor.layout.layoutMode.type === 'paginated') {
      let mostVisiblePage = 0;
      let maxRatio = 0;
      for (const [page, ratio] of editor.pageVisibility) {
        if (ratio > maxRatio) {
          maxRatio = ratio;
          mostVisiblePage = page;
        }
      }
      return `${mostVisiblePage + 1}/${editor.layout.pages.length}`;
    }
    return `${Math.round(scrollRatioV * 100)}%`;
  });

  function handleTrackClickV(e: MouseEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
    lastUserScrollInputAt = Date.now();
    showTemporarily('user');
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickY = e.clientY - rect.top - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (clickY - thumbHeight / 2) / (trackHeight - thumbHeight)));
    scrollContainer.scrollTop = ratio * maxScrollV;
  }

  function handleTrackClickH(e: MouseEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
    lastUserScrollInputAt = Date.now();
    showTemporarily('user');
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickX = e.clientX - rect.left - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (clickX - thumbWidth / 2) / (trackWidth - thumbWidth)));
    scrollContainer.scrollLeft = ratio * maxScrollH;
  }

  function handleThumbDragV(e: MouseEvent) {
    if (!scrollContainer) return;
    e.preventDefault();
    e.stopPropagation();

    isDraggingV = true;
    lastUserScrollInputAt = Date.now();
    showTemporarily('user');
    dragStartY = e.clientY;
    dragStartScrollTop = scrollContainer.scrollTop;

    const onMove = (e: MouseEvent) => {
      if (!scrollContainer) return;
      lastUserScrollInputAt = Date.now();
      const delta = ((e.clientY - dragStartY) / (trackHeight - thumbHeight)) * maxScrollV;
      scrollContainer.scrollTop = dragStartScrollTop + delta;
    };

    const onUp = () => {
      isDraggingV = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      showTemporarily('user');
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }

  function handleThumbDragH(e: MouseEvent) {
    if (!scrollContainer) return;
    e.preventDefault();
    e.stopPropagation();

    isDraggingH = true;
    lastUserScrollInputAt = Date.now();
    showTemporarily('user');
    dragStartX = e.clientX;
    dragStartScrollLeft = scrollContainer.scrollLeft;

    const onMove = (e: MouseEvent) => {
      if (!scrollContainer) return;
      lastUserScrollInputAt = Date.now();
      const delta = ((e.clientX - dragStartX) / (trackWidth - thumbWidth)) * maxScrollH;
      scrollContainer.scrollLeft = dragStartScrollLeft + delta;
    };

    const onUp = () => {
      isDraggingH = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      showTemporarily('user');
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }
</script>

{#if canScrollV && containerRect && isUserScrollVisible}
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
      opacity: isVisible ? '100' : '0',
    })}
  >
    {displayText}
  </div>
{/if}

{#if canScrollV && containerRect}
  <div
    style:top="{containerRect.top}px"
    style:right="{window.innerWidth - containerRect.right}px"
    style:height="{containerRect.height - (canScrollH ? 12 : 0)}px"
    class={css({
      pointerEvents: 'auto',
      position: 'fixed',
      zIndex: '10',
      width: '12px',
      transition: 'opacity',
      transitionDuration: '300ms',
      opacity: isVisible || isDraggingV ? (isUserScrollVisible ? '100' : '65') : '0',
    })}
    aria-controls="scroll-content"
    aria-valuemax={maxScrollV}
    aria-valuemin={0}
    aria-valuenow={scrollTop}
    onmousedown={handleTrackClickV}
    onmouseenter={handleHoverStartV}
    onmouseleave={handleHoverEndV}
    role="scrollbar"
    tabindex="-1"
  >
    <div
      style:top="{thumbTop}px"
      style:height="{thumbHeight}px"
      class={css({
        position: 'absolute',
        right: '2px',
        width: '8px',
        cursor: 'pointer',
        borderRadius: 'full',
        backgroundColor: isUserScrollVisible ? 'control.scrollbar.hover' : 'control.scrollbar.default',
        transition: 'colors',
        _hover: { backgroundColor: 'control.scrollbar.hover' },
        _active: { backgroundColor: 'control.scrollbar.hover' },
      })}
      aria-valuemax={maxScrollV}
      aria-valuemin={0}
      aria-valuenow={scrollTop}
      onmousedown={handleThumbDragV}
      role="slider"
      tabindex="-1"
    ></div>
  </div>
{/if}

{#if canScrollH && containerRect}
  <div
    style:left="{containerRect.left}px"
    style:bottom="{window.innerHeight - containerRect.bottom}px"
    style:width="{containerRect.width - (canScrollV ? 12 : 0)}px"
    class={css({
      pointerEvents: 'auto',
      position: 'fixed',
      zIndex: '10',
      height: '12px',
      transition: 'opacity',
      transitionDuration: '300ms',
      opacity: isVisible || isDraggingH ? (isUserScrollVisible ? '100' : '65') : '0',
    })}
    aria-controls="scroll-content"
    aria-orientation="horizontal"
    aria-valuemax={maxScrollH}
    aria-valuemin={0}
    aria-valuenow={scrollLeft}
    onmousedown={handleTrackClickH}
    onmouseenter={handleHoverStartH}
    onmouseleave={handleHoverEndH}
    role="scrollbar"
    tabindex="-1"
  >
    <div
      style:left="{thumbLeft}px"
      style:width="{thumbWidth}px"
      class={css({
        position: 'absolute',
        bottom: '2px',
        height: '8px',
        cursor: 'pointer',
        borderRadius: 'full',
        backgroundColor: isUserScrollVisible ? 'control.scrollbar.hover' : 'control.scrollbar.default',
        transition: 'colors',
        _hover: { backgroundColor: 'control.scrollbar.hover' },
        _active: { backgroundColor: 'control.scrollbar.hover' },
      })}
      aria-valuemax={maxScrollH}
      aria-valuemin={0}
      aria-valuenow={scrollLeft}
      onmousedown={handleThumbDragH}
      role="slider"
      tabindex="-1"
    ></div>
  </div>
{/if}
