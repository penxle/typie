<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { elementScrollViewport, windowScrollViewport } from '@typie/ui/utils';
  import { IS_MAC } from '$lib/editor/constants';
  import { EditorZoomController } from '../editor-zoom.svelte';
  import ZoomOverlay from './ZoomOverlay.svelte';
  import type { Snippet } from 'svelte';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
    active?: boolean;
    resizing?: boolean;
    useWindowScroll?: boolean;
    containerClientWidth?: number;
    containerClientHeight?: number;
    scrollLeft?: number;
    scrollTop?: number;
    renderZoom?: number;
    scrollContainer?: HTMLElement | null;
    children?: Snippet;
  };

  type PinchSession = {
    startDistance: number;
    startZoom: number;
  };

  type PinchUpdate = {
    zoom: number;
    clientX: number;
    clientY: number;
  };

  /* eslint-disable no-useless-assignment -- $bindable() defaults are used by Svelte */
  let {
    editor,
    active = true,
    resizing = false,
    useWindowScroll = false,
    containerClientWidth = $bindable(0),
    containerClientHeight = $bindable(0),
    scrollLeft = $bindable(0),
    scrollTop = $bindable(0),
    renderZoom = $bindable(1),
    scrollContainer = $bindable(null),
    children,
  }: Props = $props();
  /* eslint-enable no-useless-assignment */

  let zoomViewportWidth = $state(0);
  let pinchSession = $state<PinchSession | null>(null);
  let pinchUpdateInFlight = $state(false);
  let pinchQueuedUpdate = $state<PinchUpdate | null>(null);

  const zoom = new EditorZoomController({
    editor,
    isPaginated: () => editor.layout?.layoutMode.type === 'paginated',
    pageWidth: () => editor.layout?.pages[0]?.width ?? 0,
    viewportWidth: () => zoomViewportWidth,
    getScrollContainer: () => scrollContainer,
  });

  const isPaginated = $derived(editor.layout?.layoutMode.type === 'paginated');
  const pageWidth = $derived(editor.layout?.pages[0]?.width ?? 0);
  const displayZoom = $derived(isPaginated ? zoom.displayZoom : 1);
  const effectiveRenderZoom = $derived(isPaginated ? zoom.renderZoom : 1);

  $effect(() => {
    editor.scrollContainerEl = scrollContainer;
  });

  $effect(() => {
    if (useWindowScroll) {
      editor.scrollViewport = windowScrollViewport();
    } else if (scrollContainer) {
      editor.scrollViewport = elementScrollViewport(scrollContainer);
    } else {
      editor.scrollViewport = null;
    }
  });

  $effect(() => {
    editor.displayZoom = displayZoom;
    renderZoom = effectiveRenderZoom;
  });

  $effect(() => {
    void isPaginated;
    void pageWidth;
    void zoomViewportWidth;
    zoom.syncInitialZoom();
  });

  $effect(() => {
    void isPaginated;
    void pageWidth;
    void zoomViewportWidth;
    void zoom.displayZoom;
    zoom.clampCurrentZoomToBounds();
  });

  $effect(() => {
    return () => {
      editor.scrollContainerEl = null;
      editor.scrollViewport = null;
      zoom.destroy();
    };
  });

  const isZoomInShortcut = (event: KeyboardEvent): boolean => {
    return event.code === 'Equal' || event.code === 'NumpadAdd' || event.key === '+' || event.key === '=';
  };

  const isZoomOutShortcut = (event: KeyboardEvent): boolean => {
    return event.code === 'Minus' || event.code === 'NumpadSubtract' || event.key === '-';
  };

  const isZoomResetShortcut = (event: KeyboardEvent): boolean => {
    return event.code === 'Digit0' || event.code === 'Numpad0' || event.key === '0';
  };

  const handleBrowserZoomShortcut = (event: KeyboardEvent): void => {
    if (!active || !isPaginated) {
      return;
    }

    const hasZoomModifier = IS_MAC ? event.metaKey : event.ctrlKey;
    if (!hasZoomModifier || event.altKey) {
      return;
    }

    if (isZoomInShortcut(event)) {
      event.preventDefault();
      void zoom.zoomInByKeyboard();
      return;
    }

    if (isZoomOutShortcut(event)) {
      event.preventDefault();
      void zoom.zoomOutByKeyboard();
      return;
    }

    if (isZoomResetShortcut(event)) {
      event.preventDefault();
      void zoom.resetByKeyboard();
    }
  };

  function isTouchOnPage(touch: Touch): boolean {
    return editor.resolvePointerCoordinateFromClient(touch.clientX, touch.clientY) !== null;
  }

  function touchDistance(t1: Touch, t2: Touch): number {
    return Math.hypot(t1.clientX - t2.clientX, t1.clientY - t2.clientY);
  }

  function queuePinchUpdate(update: PinchUpdate): void {
    pinchQueuedUpdate = update;
    void flushPinchUpdates();
  }

  async function flushPinchUpdates(): Promise<void> {
    if (pinchUpdateInFlight) {
      return;
    }

    pinchUpdateInFlight = true;
    try {
      while (pinchQueuedUpdate) {
        const next = pinchQueuedUpdate;
        pinchQueuedUpdate = null;
        await zoom.zoomToClientPoint(next.zoom, next.clientX, next.clientY);
      }
    } finally {
      pinchUpdateInFlight = false;
    }
  }

  function tryStartPinch(touches: TouchList): boolean {
    if (!isPaginated || touches.length !== 2) {
      return false;
    }

    const t1 = touches.item(0);
    const t2 = touches.item(1);
    if (!t1 || !t2) {
      return false;
    }

    if (!isTouchOnPage(t1) || !isTouchOnPage(t2)) {
      return false;
    }

    const startDistance = touchDistance(t1, t2);
    if (!Number.isFinite(startDistance) || startDistance <= 0) {
      return false;
    }

    pinchSession = {
      startDistance,
      startZoom: zoom.displayZoom,
    };

    return true;
  }

  function handleTouchStartForPinch(event: TouchEvent): void {
    if (pinchSession || !isPaginated || event.touches.length !== 2) {
      return;
    }

    tryStartPinch(event.touches);
  }

  function handleTouchMoveForPinch(event: TouchEvent): void {
    if (!isPaginated || event.touches.length !== 2) {
      return;
    }

    if (!pinchSession && !tryStartPinch(event.touches)) {
      return;
    }

    const t1 = event.touches.item(0);
    const t2 = event.touches.item(1);
    if (!t1 || !t2 || !pinchSession) {
      return;
    }

    const distance = touchDistance(t1, t2);
    if (!Number.isFinite(distance) || distance <= 0) {
      return;
    }

    if (event.cancelable) {
      event.preventDefault();
    }

    queuePinchUpdate({
      zoom: pinchSession.startZoom * (distance / pinchSession.startDistance),
      clientX: (t1.clientX + t2.clientX) / 2,
      clientY: (t1.clientY + t2.clientY) / 2,
    });
  }

  function handleTouchEndForPinch(event: TouchEvent): void {
    if (event.touches.length < 2) {
      pinchSession = null;
      return;
    }

    if (event.touches.length === 2) {
      pinchSession = null;
      tryStartPinch(event.touches);
    }
  }

  function handleTouchCancelForPinch(): void {
    pinchSession = null;
  }

  $effect(() => {
    if (isPaginated) {
      return;
    }

    pinchSession = null;
    pinchQueuedUpdate = null;
  });
</script>

<svelte:window onkeydowncapture={handleBrowserZoomShortcut} />

<div
  class={css({
    position: 'relative',
    overflow: 'hidden',
  })}
>
  <div
    bind:this={scrollContainer}
    class={css({
      height: 'full',
      overflow: 'auto',
      scrollbarWidth: 'none',
      '&::-webkit-scrollbar': { display: 'none' },
    })}
    {@attach (el) => {
      let pending = false;
      let timeoutId: ReturnType<typeof setTimeout>;
      let resizeEndTimeoutId: ReturnType<typeof setTimeout>;
      const syncViewportMetrics = () => {
        if (useWindowScroll) {
          containerClientWidth = window.innerWidth;
          containerClientHeight = window.innerHeight;
          zoomViewportWidth = window.innerWidth;
          scrollLeft = window.scrollX;
          scrollTop = window.scrollY;
          return;
        }

        containerClientWidth = el.clientWidth;
        containerClientHeight = el.clientHeight;
        zoomViewportWidth = el.clientWidth;
        scrollLeft = el.scrollLeft;
        scrollTop = el.scrollTop;
      };
      const handleWindowViewportChange = () => {
        syncViewportMetrics();
      };

      el.addEventListener('touchstart', handleTouchStartForPinch, { passive: true });
      el.addEventListener('touchmove', handleTouchMoveForPinch, { passive: false });
      el.addEventListener('touchend', handleTouchEndForPinch, { passive: true });
      el.addEventListener('touchcancel', handleTouchCancelForPinch, { passive: true });
      if (useWindowScroll) {
        window.addEventListener('resize', handleWindowViewportChange);
        window.visualViewport?.addEventListener('resize', handleWindowViewportChange);
      }
      const observer = new ResizeObserver(() => {
        editor.containerResizing = true;
        clearTimeout(resizeEndTimeoutId);
        resizeEndTimeoutId = setTimeout(() => {
          editor.containerResizing = false;
        }, 300);

        if (resizing) {
          pending = true;
          return;
        }
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => {
          syncViewportMetrics();
        }, 50);
      });
      observer.observe(el);
      syncViewportMetrics();

      const teardown = $effect.root(() => {
        $effect(() => {
          if (resizing || !pending) {
            return;
          }

          pending = false;
          syncViewportMetrics();
        });
      });

      return () => {
        clearTimeout(timeoutId);
        clearTimeout(resizeEndTimeoutId);
        el.removeEventListener('touchstart', handleTouchStartForPinch);
        el.removeEventListener('touchmove', handleTouchMoveForPinch);
        el.removeEventListener('touchend', handleTouchEndForPinch);
        el.removeEventListener('touchcancel', handleTouchCancelForPinch);
        if (useWindowScroll) {
          window.removeEventListener('resize', handleWindowViewportChange);
          window.visualViewport?.removeEventListener('resize', handleWindowViewportChange);
        }
        teardown();
        observer.disconnect();
      };
    }}
    onscroll={(e) => {
      const target = e.currentTarget;
      scrollLeft = target.scrollLeft;
      scrollTop = target.scrollTop;
      editor.notifyViewportScrolled();
    }}
    onwheel={(event) => zoom.handleWheel(event)}
  >
    {@render children?.()}
  </div>

  <ZoomOverlay {displayZoom} {isPaginated} {useWindowScroll} />
</div>
