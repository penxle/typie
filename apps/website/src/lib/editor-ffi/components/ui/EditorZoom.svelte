<script lang="ts">
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { computeDocumentZoomBounds, resolveDocumentZoomIndicator, zoomEquals } from '$lib/editor-ffi/zoom';
  import { EditorZoomController } from '../editor-zoom.svelte';
  import type { Snippet } from 'svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { EditorScrollScope } from '$lib/editor-ffi/scroll.svelte';
  import type { DocumentZoomLandmark, DocumentZoomLayout } from '$lib/editor-ffi/zoom';
  import type { EditorZoomControlsRenderProps } from './EditorZoomControls.svelte';

  type Props = {
    editor: Editor;
    active?: boolean;
    layout: DocumentZoomLayout | null;
    viewportWidth: number;
    editorViewSurface: HTMLElement | undefined;
    scroll: EditorScrollScope | undefined;
    zoomControls?: Snippet<[EditorZoomControlsRenderProps]>;
  };

  type PinchSession = {
    startDistance: number;
    startZoom: number;
  };

  type PinchUpdate = {
    zoom: number;
    clientX: number;
    clientY: number;
    timestampMs: number;
  };

  type PinchContact = Pick<Touch, 'clientX' | 'clientY'>;

  let { editor, active = true, layout, viewportWidth, editorViewSurface, scroll, zoomControls }: Props = $props();

  let pinchSession = $state<PinchSession | null>(null);
  let pinchQueuedUpdate = $state<PinchUpdate | null>(null);
  let suppressPinchUntilAllUp = false;
  let pinchFlushPromise: Promise<void> | null = null;
  let boundaryAttemptRequest = $state(0);
  let boundaryAttemptLandmark = $state<DocumentZoomLandmark | null>(null);
  const zoom = new EditorZoomController({
    editor,
    layout: () => layout,
    viewportWidth: () => viewportWidth,
    getScrollViewport: () => editor.scrollViewport,
    resolvePendingViewportAnchor: () => scroll?.resolvePendingViewportZoomAnchor() ?? null,
    resolveViewportAnchor: () => scroll?.resolveViewportZoomAnchor() ?? null,
    beginViewportZoom: (point) => scroll?.beginViewportZoomAt(point),
    updateViewportZoomAttachment: (desiredScroll) => scroll?.updateViewportZoomAttachment(desiredScroll),
  });

  const zoomEnabled = $derived(layout !== null);
  const displayZoom = $derived(zoomEnabled ? zoom.displayZoom : 1);
  const renderZoom = $derived(zoomEnabled ? zoom.renderZoom : 1);
  const indicatorZoom = $derived(layout ? resolveDocumentZoomIndicator(displayZoom, layout) : 1);
  const landmark = $derived(zoom.landmark);
  const bounds = $derived(layout ? computeDocumentZoomBounds(layout) : null);
  const atMinimum = $derived(bounds ? zoomEquals(indicatorZoom, bounds.min) : false);
  const atMaximum = $derived(bounds ? zoomEquals(indicatorZoom, bounds.max) : false);
  const toggleTargetLandmark = $derived(zoom.indicatorToggleTargetLandmark);

  function requestBoundaryAttempt(nextLandmark: 'minimum' | 'maximum') {
    boundaryAttemptLandmark = nextLandmark;
    boundaryAttemptRequest += 1;
  }

  async function zoomIn(): Promise<boolean> {
    const changed = await zoom.zoomInByKeyboard();
    if (!changed && atMaximum) requestBoundaryAttempt('maximum');
    return changed;
  }

  async function zoomOut(): Promise<boolean> {
    const changed = await zoom.zoomOutByKeyboard();
    if (!changed && atMinimum) requestBoundaryAttempt('minimum');
    return changed;
  }

  async function toggleZoom(): Promise<void> {
    await zoom.toggleZoomByIndicator();
  }

  $effect(() => {
    editor.displayZoom = displayZoom;
    editor.commitRenderZoom(renderZoom);
  });

  $effect(() => {
    void layout;
    void viewportWidth;
    zoom.syncInitialZoom();
  });

  $effect(() => {
    void layout;
    void viewportWidth;
    void zoom.displayZoom;
    zoom.clampCurrentZoomToBounds();
  });

  $effect(() => {
    return () => {
      zoom.destroy();
      editor.displayZoom = 1;
      editor.commitRenderZoom(1);
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
    if (!active || !zoomEnabled) return;

    const editorPane = editor.inputEl?.closest<HTMLElement>('[data-pane-id]');
    const targetPane = event.target instanceof Element ? event.target.closest<HTMLElement>('[data-pane-id]') : null;
    if (event.target !== editor.inputEl && (!editorPane || targetPane !== editorPane)) return;

    const hasZoomModifier = IS_MAC ? event.metaKey : event.ctrlKey;
    if (!hasZoomModifier || event.altKey) return;

    if (isZoomInShortcut(event)) {
      event.preventDefault();
      void zoomIn();
      return;
    }

    if (isZoomOutShortcut(event)) {
      event.preventDefault();
      void zoomOut();
      return;
    }

    if (isZoomResetShortcut(event)) {
      event.preventDefault();
      void zoom.resetByKeyboard();
    }
  };

  function isTouchOnPage(touch: PinchContact): boolean {
    return editor.clientToLocal(touch.clientX, touch.clientY) !== null;
  }

  function touchDistance(t1: PinchContact, t2: PinchContact): number {
    return Math.hypot(t1.clientX - t2.clientX, t1.clientY - t2.clientY);
  }

  function queuePinchUpdate(update: PinchUpdate): void {
    pinchQueuedUpdate = update;
    pinchFlushPromise ??= flushPinchUpdates().finally(() => {
      pinchFlushPromise = null;
    });
  }

  async function flushPinchUpdates(): Promise<void> {
    while (pinchQueuedUpdate) {
      const next = pinchQueuedUpdate;
      pinchQueuedUpdate = null;
      await zoom.updateDirectZoom('touch', next.zoom, next.clientX, next.clientY, next.timestampMs);
    }
  }

  async function finishPinchZoom(): Promise<void> {
    await pinchFlushPromise;
    await zoom.releaseDirectZoom('touch');
  }

  function cancelPinchAndSuppressUntilAllUp(remainingTouches: number): void {
    pinchSession = null;
    pinchQueuedUpdate = null;
    suppressPinchUntilAllUp = remainingTouches > 0;
    zoom.cancelDirectZoom('touch');
  }

  function tryStartPinchWithContacts(t1: PinchContact, t2: PinchContact, timestampMs: number): boolean {
    if (!zoomEnabled) return false;
    if (!isTouchOnPage(t1) || !isTouchOnPage(t2)) {
      return false;
    }

    const startDistance = touchDistance(t1, t2);
    if (!Number.isFinite(startDistance) || startDistance <= 0) {
      return false;
    }

    const clientX = (t1.clientX + t2.clientX) / 2;
    const clientY = (t1.clientY + t2.clientY) / 2;
    if (!zoom.beginDirectZoom('touch', clientX, clientY, timestampMs)) {
      return false;
    }

    pinchSession = {
      startDistance,
      startZoom: zoom.displayZoom,
    };

    return true;
  }

  function tryStartPinch(touches: TouchList, timestampMs: number): boolean {
    if (touches.length !== 2) return false;
    const t1 = touches.item(0);
    const t2 = touches.item(1);
    return t1 !== null && t2 !== null && tryStartPinchWithContacts(t1, t2, timestampMs);
  }

  function handleTouchStartForPinch(event: TouchEvent): void {
    if (!zoomEnabled) {
      return;
    }

    if (suppressPinchUntilAllUp) return;

    if (pinchSession && event.touches.length > 2) {
      cancelPinchAndSuppressUntilAllUp(event.touches.length);
      return;
    }

    if (event.touches.length === 1) {
      zoom.interruptForDirectPan();
      return;
    }

    if (pinchSession || event.touches.length !== 2) return;

    tryStartPinch(event.touches, event.timeStamp);
  }

  function handleTouchMoveForPinch(event: TouchEvent): void {
    if (suppressPinchUntilAllUp) {
      if (event.cancelable) event.preventDefault();
      return;
    }

    if (!zoomEnabled || event.touches.length !== 2) {
      return;
    }

    if (!pinchSession && !tryStartPinch(event.touches, event.timeStamp)) {
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
      timestampMs: event.timeStamp,
    });
  }

  function handleTouchEndForPinch(event: TouchEvent): void {
    if (suppressPinchUntilAllUp) {
      if (event.touches.length === 0) suppressPinchUntilAllUp = false;
      return;
    }

    if (!pinchSession) return;
    if (event.touches.length >= 2) {
      cancelPinchAndSuppressUntilAllUp(event.touches.length);
      return;
    }

    pinchSession = null;
    suppressPinchUntilAllUp = event.touches.length > 0;
    void finishPinchZoom();
  }

  function handleTouchCancelForPinch(event: TouchEvent): void {
    cancelPinchAndSuppressUntilAllUp(event.touches.length);
  }

  $effect(() => {
    const wheelTarget = editorViewSurface;
    const touchTarget = editor.scrollViewport?.target;
    if (!wheelTarget || !touchTarget) return;

    const handleWheelForZoom = (event: Event) => {
      void zoom.handleWheel(event as WheelEvent);
    };
    const handleTouchStart = (event: Event) => {
      handleTouchStartForPinch(event as TouchEvent);
    };
    const handleTouchMove = (event: Event) => {
      handleTouchMoveForPinch(event as TouchEvent);
    };
    const handleTouchEnd = (event: Event) => {
      handleTouchEndForPinch(event as TouchEvent);
    };
    const handleTouchCancel = (event: Event) => {
      handleTouchCancelForPinch(event as TouchEvent);
    };

    wheelTarget.addEventListener('wheel', handleWheelForZoom, { capture: true, passive: false });
    touchTarget.addEventListener('touchstart', handleTouchStart, { passive: true });
    touchTarget.addEventListener('touchmove', handleTouchMove, { passive: false });
    touchTarget.addEventListener('touchend', handleTouchEnd, { passive: true });
    touchTarget.addEventListener('touchcancel', handleTouchCancel, { passive: true });

    return () => {
      pinchSession = null;
      pinchQueuedUpdate = null;
      suppressPinchUntilAllUp = false;
      zoom.cancelDirectZoom('touch');
      wheelTarget.removeEventListener('wheel', handleWheelForZoom, { capture: true });
      touchTarget.removeEventListener('touchstart', handleTouchStart);
      touchTarget.removeEventListener('touchmove', handleTouchMove);
      touchTarget.removeEventListener('touchend', handleTouchEnd);
      touchTarget.removeEventListener('touchcancel', handleTouchCancel);
    };
  });

  $effect(() => {
    if (zoomEnabled) {
      return;
    }

    pinchSession = null;
    pinchQueuedUpdate = null;
    suppressPinchUntilAllUp = false;
    zoom.cancelDirectZoom();
  });
</script>

<svelte:window onkeydowncapture={handleBrowserZoomShortcut} />

{@render zoomControls?.({
  atMaximum,
  atMinimum,
  boundaryAttemptLandmark,
  boundaryAttemptRequest,
  displayZoom,
  enabled: zoomEnabled,
  indicatorZoom,
  landmark,
  onToggleZoom: toggleZoom,
  onZoomIn: zoomIn,
  onZoomOut: zoomOut,
  toggleTargetLandmark,
})}
