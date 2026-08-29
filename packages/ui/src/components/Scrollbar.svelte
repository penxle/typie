<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pointerCapture } from '../actions';

  const HIDE_DELAY = 1000;
  const MIN_THUMB_SIZE = 30;
  const TRACK_PADDING = 2;

  const AXES = {
    horizontal: {
      contentSize: 'scrollWidth',
      pointerPosition: (event: PointerEvent) => event.clientX,
      rectStart: (rect: DOMRect) => rect.left,
      scrollPosition: 'scrollLeft',
      viewportSize: 'clientWidth',
    },
    vertical: {
      contentSize: 'scrollHeight',
      pointerPosition: (event: PointerEvent) => event.clientY,
      rectStart: (rect: DOMRect) => rect.top,
      scrollPosition: 'scrollTop',
      viewportSize: 'clientHeight',
    },
  } as const;

  const SIZES = {
    sm: { hitLane: 8, paintedThumb: 3, paintedThumbInset: 2 },
    md: { hitLane: 12, paintedThumb: 6, paintedThumbInset: 3 },
    lg: { hitLane: 12, paintedThumb: 10, paintedThumbInset: 1 },
  } as const;

  type Orientation = keyof typeof AXES;
  type ScrollbarSize = keyof typeof SIZES;

  type Props = {
    controls: string;
    label: string;
    orientation: Orientation;
    scrollContainer: HTMLElement | undefined;
    size: ScrollbarSize;
  };

  type Metrics = {
    contentSize: number;
    scrollPosition: number;
    viewportSize: number;
  };

  type DragSession = {
    maxScroll: number;
    startPointer: number;
    startScroll: number;
    thumbTravel: number;
  };

  let { controls, label, orientation, scrollContainer, size }: Props = $props();

  let metrics = $state<Metrics>({ contentSize: 0, scrollPosition: 0, viewportSize: 0 });
  let transientVisible = $state(false);
  let dragging = $state(false);
  let containerHovered = $state(false);
  let thumbHovered = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const axis = $derived(AXES[orientation]);
  const dimensions = $derived(SIZES[size]);
  const vertical = $derived(orientation === 'vertical');

  const geometry = $derived.by(() => {
    const maxScroll = Math.max(0, metrics.contentSize - metrics.viewportSize);
    const trackSize = Math.max(0, metrics.viewportSize - TRACK_PADDING * 2);
    const canScroll = scrollContainer !== undefined && maxScroll > 0 && trackSize > 0;
    const idealThumbSize = metrics.contentSize > 0 ? (metrics.viewportSize / metrics.contentSize) * trackSize : trackSize;
    const thumbSize = Math.min(trackSize, Math.max(MIN_THUMB_SIZE, idealThumbSize));
    const thumbTravel = Math.max(0, trackSize - thumbSize);
    const ratio = maxScroll > 0 ? Math.max(0, Math.min(1, metrics.scrollPosition / maxScroll)) : 0;

    return {
      canScroll,
      maxScroll,
      thumbPos: TRACK_PADDING + ratio * thumbTravel,
      thumbSize,
      thumbTravel,
    };
  });

  const visible = $derived(containerHovered || transientVisible || dragging);

  $effect(() => {
    if (!geometry.canScroll) thumbHovered = false;
  });

  function setScrollPosition(value: number) {
    if (!scrollContainer) return;
    scrollContainer[axis.scrollPosition] = value;
  }

  function showTemporarily() {
    transientVisible = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (transientVisible = false), HIDE_DELAY);
  }

  function handlePointerEnter() {
    containerHovered = true;
    clearTimeout(hideTimer);
    transientVisible = false;
  }

  function handlePointerLeave() {
    if (!containerHovered) return;
    containerHovered = false;
    clearTimeout(hideTimer);
    transientVisible = false;
  }

  $effect(() => {
    const element = scrollContainer;
    if (!element) return;
    containerHovered = false;

    const sync = () => {
      metrics = {
        contentSize: element[axis.contentSize],
        scrollPosition: element[axis.scrollPosition],
        viewportSize: element[axis.viewportSize],
      };
    };

    const handleScroll = () => {
      sync();
      showTemporarily();
    };

    const resizeObserver = new ResizeObserver(sync);
    const observeCurrentElements = () => {
      resizeObserver.disconnect();
      resizeObserver.observe(element);
      for (const child of element.children) resizeObserver.observe(child);
    };
    const mutationObserver = new MutationObserver(() => {
      observeCurrentElements();
      sync();
    });

    observeCurrentElements();
    mutationObserver.observe(element, { childList: true });
    element.addEventListener('scroll', handleScroll);
    element.addEventListener('pointerenter', handlePointerEnter);
    element.addEventListener('pointerleave', handlePointerLeave);
    sync();

    return () => {
      element.removeEventListener('scroll', handleScroll);
      element.removeEventListener('pointerenter', handlePointerEnter);
      element.removeEventListener('pointerleave', handlePointerLeave);
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
    const click = axis.pointerPosition(event) - axis.rectStart(track.getBoundingClientRect()) - TRACK_PADDING;
    const ratio = Math.max(0, Math.min(1, (click - geometry.thumbSize / 2) / geometry.thumbTravel));
    setScrollPosition(ratio * geometry.maxScroll);
    showTemporarily();
  }

  function startDrag(event: PointerEvent): DragSession | null {
    if (!scrollContainer || dragging || !event.isPrimary || event.button !== 0 || geometry.thumbTravel === 0) return null;
    event.preventDefault();
    event.stopPropagation();

    dragging = true;
    clearTimeout(hideTimer);
    return {
      maxScroll: geometry.maxScroll,
      startPointer: axis.pointerPosition(event),
      startScroll: scrollContainer[axis.scrollPosition],
      thumbTravel: geometry.thumbTravel,
    };
  }

  function moveDrag(session: DragSession, event: PointerEvent) {
    if (!scrollContainer || !dragging) return;
    const delta = ((axis.pointerPosition(event) - session.startPointer) / session.thumbTravel) * session.maxScroll;
    setScrollPosition(session.startScroll + delta);
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
    style:top={vertical ? '0' : undefined}
    style:right="0"
    style:bottom="0"
    style:left={vertical ? undefined : '0'}
    style:width={vertical ? `${dimensions.hitLane}px` : undefined}
    style:height={vertical ? undefined : `${dimensions.hitLane}px`}
    style:opacity={visible ? 1 : 0}
    style:transition-property="opacity"
    style:transition-duration={visible ? '300ms' : '600ms'}
    style:transition-timing-function="cubic-bezier(0.4, 0, 0.2, 1)"
    class={css({
      position: 'absolute',
      zIndex: '1',
      pointerEvents: 'auto',
      touchAction: 'none',
    })}
    aria-controls={controls}
    aria-label={label}
    aria-orientation={orientation}
    aria-valuemax={geometry.maxScroll}
    aria-valuemin={0}
    aria-valuenow={metrics.scrollPosition}
    onpointerdown={jumpTo}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    role="scrollbar"
    tabindex="-1"
  >
    <div
      style:top={vertical ? `${geometry.thumbPos}px` : undefined}
      style:right={vertical ? '0' : undefined}
      style:bottom={vertical ? undefined : '0'}
      style:left={vertical ? undefined : `${geometry.thumbPos}px`}
      style:width={vertical ? `${dimensions.hitLane}px` : `${geometry.thumbSize}px`}
      style:height={vertical ? `${geometry.thumbSize}px` : `${dimensions.hitLane}px`}
      class={css({ position: 'absolute', cursor: 'pointer' })}
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
        style:top={vertical ? '0' : undefined}
        style:right={vertical ? `${dimensions.paintedThumbInset}px` : '0'}
        style:bottom={vertical ? '0' : `${dimensions.paintedThumbInset}px`}
        style:left={vertical ? undefined : '0'}
        style:width={vertical ? `${dimensions.paintedThumb}px` : undefined}
        style:height={vertical ? undefined : `${dimensions.paintedThumb}px`}
        class={css({
          position: 'absolute',
          pointerEvents: 'none',
          borderRadius: 'full',
          backgroundColor: dragging || thumbHovered ? 'surface.inverse/30' : 'surface.inverse/18',
          transition: 'colors',
        })}
      ></div>
    </div>
  </div>
{/if}
