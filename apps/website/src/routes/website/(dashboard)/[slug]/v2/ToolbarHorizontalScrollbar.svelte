<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pointerCapture } from '@typie/ui/actions';

  const HIDE_DELAY = 1000;
  const MIN_THUMB_SIZE = 30;
  const TRACK_PADDING = 2;

  type Props = {
    controls: string;
    scrollContainer: HTMLElement | undefined;
  };

  let { controls, scrollContainer }: Props = $props();

  type Metrics = {
    scrollLeft: number;
    scrollWidth: number;
    clientWidth: number;
  };

  type DragSession = {
    startPointer: number;
    startScroll: number;
    maxScroll: number;
    thumbTravel: number;
  };

  let metrics = $state<Metrics>({ scrollLeft: 0, scrollWidth: 0, clientWidth: 0 });
  let transientVisible = $state(false);
  let dragging = $state(false);
  let rowHovered = $state(false);
  let thumbHovered = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const geometry = $derived.by(() => {
    const maxScroll = Math.max(0, metrics.scrollWidth - metrics.clientWidth);
    const trackSize = Math.max(0, metrics.clientWidth - TRACK_PADDING * 2);
    const canScroll = maxScroll > 0 && trackSize > 0;
    const idealThumbSize = metrics.scrollWidth > 0 ? (metrics.clientWidth / metrics.scrollWidth) * trackSize : trackSize;
    const thumbSize = Math.min(trackSize, Math.max(MIN_THUMB_SIZE, idealThumbSize));
    const thumbTravel = Math.max(0, trackSize - thumbSize);
    const ratio = maxScroll > 0 ? Math.max(0, Math.min(1, metrics.scrollLeft / maxScroll)) : 0;

    return {
      canScroll,
      maxScroll,
      thumbSize,
      thumbTravel,
      thumbPos: TRACK_PADDING + ratio * thumbTravel,
      trackSize,
    };
  });

  const visible = $derived(rowHovered || transientVisible || dragging);

  $effect(() => {
    if (!geometry.canScroll) thumbHovered = false;
  });

  function showTemporarily() {
    transientVisible = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (transientVisible = false), HIDE_DELAY);
  }

  function handlePointerEnter() {
    rowHovered = true;
    clearTimeout(hideTimer);
    transientVisible = false;
  }

  function handlePointerLeave() {
    if (!rowHovered) return;
    rowHovered = false;
    showTemporarily();
  }

  $effect(() => {
    const el = scrollContainer;
    if (!el) return;
    rowHovered = false;

    const sync = () => {
      metrics = {
        scrollLeft: el.scrollLeft,
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
      };
    };

    const handleScroll = () => {
      sync();
      showTemporarily();
    };

    const resizeObserver = new ResizeObserver(sync);
    const observeCurrentElements = () => {
      resizeObserver.disconnect();
      resizeObserver.observe(el);
      for (const child of el.children) resizeObserver.observe(child);
    };
    const mutationObserver = new MutationObserver(() => {
      observeCurrentElements();
      sync();
    });

    observeCurrentElements();
    mutationObserver.observe(el, { childList: true });
    el.addEventListener('scroll', handleScroll);
    el.addEventListener('pointerenter', handlePointerEnter);
    el.addEventListener('pointerleave', handlePointerLeave);
    sync();

    return () => {
      el.removeEventListener('scroll', handleScroll);
      el.removeEventListener('pointerenter', handlePointerEnter);
      el.removeEventListener('pointerleave', handlePointerLeave);
      mutationObserver.disconnect();
      resizeObserver.disconnect();
      clearTimeout(hideTimer);
    };
  });

  function jumpTo(event: PointerEvent) {
    if (!scrollContainer || event.target !== event.currentTarget || !event.isPrimary || event.button !== 0 || geometry.thumbTravel === 0)
      return;
    event.preventDefault();
    event.stopPropagation();

    const track = event.currentTarget as HTMLElement;
    const click = event.clientX - track.getBoundingClientRect().left - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (click - geometry.thumbSize / 2) / geometry.thumbTravel));
    scrollContainer.scrollLeft = ratio * geometry.maxScroll;
    showTemporarily();
  }

  function startDrag(event: PointerEvent): DragSession | null {
    if (!scrollContainer || dragging || !event.isPrimary || event.button !== 0 || geometry.thumbTravel === 0) return null;
    event.preventDefault();
    event.stopPropagation();

    dragging = true;
    clearTimeout(hideTimer);
    return {
      startPointer: event.clientX,
      startScroll: scrollContainer.scrollLeft,
      maxScroll: geometry.maxScroll,
      thumbTravel: geometry.thumbTravel,
    };
  }

  function moveDrag(session: DragSession, event: PointerEvent) {
    if (!scrollContainer || !dragging) return;
    const delta = ((event.clientX - session.startPointer) / session.thumbTravel) * session.maxScroll;
    scrollContainer.scrollLeft = session.startScroll + delta;
  }

  function finishDrag() {
    if (!dragging) return;
    dragging = false;
    showTemporarily();
  }

  function endDrag(session: DragSession, event: PointerEvent) {
    moveDrag(session, event);
    finishDrag();
  }
</script>

{#if geometry.canScroll}
  <div
    style:opacity={visible ? 1 : 0}
    class={css({
      position: 'absolute',
      zIndex: '1',
      right: '0',
      bottom: '0',
      left: '0',
      height: '8px',
      pointerEvents: 'auto',
      touchAction: 'none',
      transition: 'opacity',
      transitionDuration: '300ms',
    })}
    aria-controls={controls}
    aria-label="툴바 가로 스크롤"
    aria-orientation="horizontal"
    aria-valuemax={geometry.maxScroll}
    aria-valuemin={0}
    aria-valuenow={metrics.scrollLeft}
    onpointerdown={jumpTo}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    role="scrollbar"
    tabindex="-1"
  >
    <div
      style:left="{geometry.thumbPos}px"
      style:width="{geometry.thumbSize}px"
      class={css({
        position: 'absolute',
        bottom: '0',
        height: '8px',
        cursor: 'pointer',
      })}
      onpointerenter={() => (thumbHovered = true)}
      onpointerleave={() => (thumbHovered = false)}
      role="presentation"
      use:pointerCapture={{
        start: startDrag,
        move: moveDrag,
        end: endDrag,
        cancel: finishDrag,
      }}
    >
      <div
        class={css({
          position: 'absolute',
          right: '0',
          bottom: '2px',
          left: '0',
          height: '3px',
          pointerEvents: 'none',
          borderRadius: 'full',
          backgroundColor: dragging || thumbHovered ? 'surface.inverse/45' : 'surface.inverse/22',
          transition: 'colors',
        })}
      ></div>
    </div>
  </div>
{/if}
