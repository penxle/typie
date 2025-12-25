<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditor } from '$lib/editor/context';

  type Props = {
    scrollContainer: HTMLElement | null;
  };

  const HIDE_DELAY = 1000;
  const MIN_THUMB_SIZE = 30;
  const TRACK_PADDING = 2;
  const INDICATOR_HEIGHT = 24;
  const INDICATOR_GAP = 14;

  let { scrollContainer }: Props = $props();

  const editor = getEditor();

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
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
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

    const handleUserScroll = () => {
      showTemporarily();
    };

    const resizeObserver = new ResizeObserver(syncState);
    resizeObserver.observe(scrollContainer);

    scrollContainer.addEventListener('scroll', syncState);
    scrollContainer.addEventListener('wheel', handleUserScroll, { passive: true });
    scrollContainer.addEventListener('touchmove', handleUserScroll, { passive: true });
    syncState();

    return () => {
      scrollContainer.removeEventListener('scroll', syncState);
      scrollContainer.removeEventListener('wheel', handleUserScroll);
      scrollContainer.removeEventListener('touchmove', handleUserScroll);
      resizeObserver.disconnect();
      clearTimeout(hideTimer);
    };
  });

  function showTemporarily() {
    isVisible = true;
    clearTimeout(hideTimer);
    if (!isDraggingV && !isDraggingH && !isHoveringV && !isHoveringH) {
      hideTimer = setTimeout(() => (isVisible = false), HIDE_DELAY);
    }
  }

  function handleHoverStartV() {
    isHoveringV = true;
    isVisible = true;
    clearTimeout(hideTimer);
  }

  function handleHoverEndV() {
    isHoveringV = false;
    showTemporarily();
  }

  function handleHoverStartH() {
    isHoveringH = true;
    isVisible = true;
    clearTimeout(hideTimer);
  }

  function handleHoverEndH() {
    isHoveringH = false;
    showTemporarily();
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
      return `${mostVisiblePage + 1}/${editor.layout.pageCount}`;
    }
    return `${Math.round(scrollRatioV * 100)}%`;
  });

  function handleTrackClickV(e: MouseEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickY = e.clientY - rect.top - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (clickY - thumbHeight / 2) / (trackHeight - thumbHeight)));
    scrollContainer.scrollTop = ratio * maxScrollV;
  }

  function handleTrackClickH(e: MouseEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
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
    dragStartY = e.clientY;
    dragStartScrollTop = scrollContainer.scrollTop;

    const onMove = (e: MouseEvent) => {
      if (!scrollContainer) return;
      const delta = ((e.clientY - dragStartY) / (trackHeight - thumbHeight)) * maxScrollV;
      scrollContainer.scrollTop = dragStartScrollTop + delta;
    };

    const onUp = () => {
      isDraggingV = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      showTemporarily();
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }

  function handleThumbDragH(e: MouseEvent) {
    if (!scrollContainer) return;
    e.preventDefault();
    e.stopPropagation();

    isDraggingH = true;
    dragStartX = e.clientX;
    dragStartScrollLeft = scrollContainer.scrollLeft;

    const onMove = (e: MouseEvent) => {
      if (!scrollContainer) return;
      const delta = ((e.clientX - dragStartX) / (trackWidth - thumbWidth)) * maxScrollH;
      scrollContainer.scrollLeft = dragStartScrollLeft + delta;
    };

    const onUp = () => {
      isDraggingH = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      showTemporarily();
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }
</script>

{#if canScrollV && containerRect}
  <div
    style:top="{indicatorTop}px"
    style:right="{indicatorRight}px"
    class={css({
      pointerEvents: 'none',
      position: 'fixed',
      zIndex: '20',
      borderRadius: '4px',
      backgroundColor: 'black/65',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '11px',
      whiteSpace: 'nowrap',
      color: 'white',
      fontVariantNumeric: 'tabular-nums',
      transition: 'opacity',
      transitionDuration: '300ms',
      ...(!isVisible && { opacity: '0' }),
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
      ...(!isVisible && !isDraggingV && { opacity: '0' }),
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
        backgroundColor: 'black/50',
        transition: 'colors',
        _hover: { backgroundColor: 'black/70' },
        _active: { backgroundColor: 'black/80' },
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
      ...(!isVisible && !isDraggingH && { opacity: '0' }),
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
        backgroundColor: 'black/50',
        transition: 'colors',
        _hover: { backgroundColor: 'black/70' },
        _active: { backgroundColor: 'black/80' },
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
