<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditor } from '$lib/editor/context';

  type Props = {
    scrollContainer: HTMLElement | null;
  };

  const HIDE_DELAY = 1000;
  const MIN_THUMB_HEIGHT = 30;
  const TRACK_PADDING = 2;
  const INDICATOR_HEIGHT = 24;
  const INDICATOR_GAP = 14;

  let { scrollContainer }: Props = $props();

  const editor = getEditor();

  let scrollTop = $state(0);
  let scrollHeight = $state(0);
  let clientHeight = $state(0);
  let containerRect = $state<DOMRect | null>(null);
  let isDragging = $state(false);
  let isHovering = $state(false);
  let isVisible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let dragStartY = 0;
  let dragStartScrollTop = 0;

  $effect(() => {
    if (!scrollContainer) return;

    const syncState = () => {
      scrollTop = scrollContainer.scrollTop;
      scrollHeight = scrollContainer.scrollHeight;
      clientHeight = scrollContainer.clientHeight;
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
    if (!isDragging && !isHovering) {
      hideTimer = setTimeout(() => (isVisible = false), HIDE_DELAY);
    }
  }

  function handleHoverStart() {
    isHovering = true;
    isVisible = true;
    clearTimeout(hideTimer);
  }

  function handleHoverEnd() {
    isHovering = false;
    showTemporarily();
  }

  const canScroll = $derived(scrollHeight > clientHeight);
  const maxScroll = $derived(scrollHeight - clientHeight);
  const scrollRatio = $derived(maxScroll > 0 ? scrollTop / maxScroll : 0);

  const trackHeight = $derived(clientHeight - TRACK_PADDING * 2);
  const thumbHeight = $derived(Math.max(MIN_THUMB_HEIGHT, (clientHeight / scrollHeight) * trackHeight));
  const thumbTop = $derived(TRACK_PADDING + scrollRatio * (trackHeight - thumbHeight));

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
    return `${Math.round(scrollRatio * 100)}%`;
  });

  function handleTrackClick(e: MouseEvent) {
    if (!scrollContainer || e.target !== e.currentTarget) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickY = e.clientY - rect.top - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (clickY - thumbHeight / 2) / (trackHeight - thumbHeight)));
    scrollContainer.scrollTop = ratio * maxScroll;
  }

  function handleThumbDrag(e: MouseEvent) {
    if (!scrollContainer) return;
    e.preventDefault();
    e.stopPropagation();

    isDragging = true;
    dragStartY = e.clientY;
    dragStartScrollTop = scrollContainer.scrollTop;

    const onMove = (e: MouseEvent) => {
      if (!scrollContainer) return;
      const delta = ((e.clientY - dragStartY) / (trackHeight - thumbHeight)) * maxScroll;
      scrollContainer.scrollTop = dragStartScrollTop + delta;
    };

    const onUp = () => {
      isDragging = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      showTemporarily();
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }
</script>

{#if canScroll && containerRect}
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

  <div
    style:top="{containerRect.top}px"
    style:right="{window.innerWidth - containerRect.right}px"
    style:height="{containerRect.height}px"
    class={css({
      pointerEvents: 'auto',
      position: 'fixed',
      zIndex: '10',
      width: '12px',
      transition: 'opacity',
      transitionDuration: '300ms',
      ...(!isVisible && !isDragging && { opacity: '0' }),
    })}
    aria-controls="scroll-content"
    aria-valuemax={maxScroll}
    aria-valuemin={0}
    aria-valuenow={scrollTop}
    onmousedown={handleTrackClick}
    onmouseenter={handleHoverStart}
    onmouseleave={handleHoverEnd}
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
      aria-valuemax={maxScroll}
      aria-valuemin={0}
      aria-valuenow={scrollTop}
      onmousedown={handleThumbDrag}
      role="slider"
      tabindex="-1"
    ></div>
  </div>
{/if}
