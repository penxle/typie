<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { EditorZoomController } from '../editor-zoom.svelte';
  import ZoomOverlay from './ZoomOverlay.svelte';
  import type { Snippet } from 'svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = {
    editor: Editor;
    active?: boolean;
    isPaginated: boolean;
    pageWidth: number;
    viewportWidth: number;
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

  let { editor, active = true, isPaginated, pageWidth, viewportWidth, children }: Props = $props();

  let pinchSession = $state<PinchSession | null>(null);
  let pinchUpdateInFlight = $state(false);
  let pinchQueuedUpdate = $state<PinchUpdate | null>(null);
  const scrollContainer = $derived(editor.scrollContainerEl);

  const zoom = new EditorZoomController({
    editor,
    isPaginated: () => isPaginated,
    pageWidth: () => pageWidth,
    viewportWidth: () => viewportWidth,
    getScrollViewport: () => editor.scrollViewport,
  });

  const displayZoom = $derived(isPaginated ? zoom.displayZoom : 1);
  const renderZoom = $derived(isPaginated ? zoom.renderZoom : 1);

  $effect(() => {
    editor.displayZoom = displayZoom;
    editor.setRenderZoom(renderZoom);
  });

  $effect(() => {
    void isPaginated;
    void pageWidth;
    void viewportWidth;
    zoom.syncInitialZoom();
  });

  $effect(() => {
    void isPaginated;
    void pageWidth;
    void viewportWidth;
    void zoom.displayZoom;
    zoom.clampCurrentZoomToBounds();
  });

  $effect(() => {
    return () => {
      zoom.destroy();
      editor.displayZoom = 1;
      editor.setRenderZoom(1);
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
    return editor.clientToLocal(touch.clientX, touch.clientY) !== null;
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
    const target = editor.scrollViewport?.target;
    if (!target) return;

    const handleWheelForZoom = (event: Event) => {
      if (!active) return;
      void zoom.handleWheel(event as WheelEvent);
    };
    const handleTouchStart = (event: Event) => {
      if (!active) return;
      handleTouchStartForPinch(event as TouchEvent);
    };
    const handleTouchMove = (event: Event) => {
      if (!active) return;
      handleTouchMoveForPinch(event as TouchEvent);
    };
    const handleTouchEnd = (event: Event) => {
      if (!active) return;
      handleTouchEndForPinch(event as TouchEvent);
    };
    const handleTouchCancel = () => {
      if (!active) return;
      handleTouchCancelForPinch();
    };

    target.addEventListener('wheel', handleWheelForZoom, { capture: true, passive: false });
    target.addEventListener('touchstart', handleTouchStart, { passive: true });
    target.addEventListener('touchmove', handleTouchMove, { passive: false });
    target.addEventListener('touchend', handleTouchEnd, { passive: true });
    target.addEventListener('touchcancel', handleTouchCancel, { passive: true });

    return () => {
      target.removeEventListener('wheel', handleWheelForZoom, { capture: true });
      target.removeEventListener('touchstart', handleTouchStart);
      target.removeEventListener('touchmove', handleTouchMove);
      target.removeEventListener('touchend', handleTouchEnd);
      target.removeEventListener('touchcancel', handleTouchCancel);
    };
  });

  $effect(() => {
    if (!(!active || !isPaginated)) {
      return;
    }

    pinchSession = null;
    pinchQueuedUpdate = null;
  });
</script>

<svelte:window onkeydowncapture={handleBrowserZoomShortcut} />

<div class={css({ display: 'contents' })}>
  {@render children?.()}
</div>

<ZoomOverlay {displayZoom} {isPaginated} {scrollContainer} />
