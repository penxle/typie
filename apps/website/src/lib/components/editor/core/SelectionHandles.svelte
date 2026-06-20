<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { getOrderedSelectionHandles } from '$lib/editor/touch-gesture.svelte';
  import type { SelectionHandleKind } from '$lib/editor/touch-gesture.svelte';

  const HANDLE_RADIUS = 8;
  const STEM_WIDTH = 2;
  const TOUCH_TARGET_SIZE = 44;

  type HandleVisual = {
    left: number;
    top: number;
    touchHeight: number;
    paintLeft: number;
    paintTop: number;
    stemHeight: number;
  };

  const { editor } = getEditorContext();

  const touchEnabled = typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0;

  const fromHandle = $derived.by(() => {
    return getHandleVisual('from');
  });

  const toHandle = $derived.by(() => {
    return getHandleVisual('to');
  });

  function isTouchLikePointer(event: PointerEvent): boolean {
    return event.pointerType === 'touch';
  }

  function getHandleVisual(type: SelectionHandleKind): HandleVisual | null {
    if (!editor.readOnly || !touchEnabled) {
      return null;
    }

    const handles = getOrderedSelectionHandles(editor.selection);
    if (!handles) {
      return null;
    }

    const endpoint = type === 'from' ? handles.from : handles.to;
    const pageEl = editor.pageContainerEls[endpoint.pageIdx];
    const containerEl = editor.extensionArea.containerEl;
    if (!pageEl || !containerEl) {
      return null;
    }

    const pageRect = pageEl.getBoundingClientRect();
    const containerRect = containerEl.getBoundingClientRect();
    const zoom = editor.layout?.layoutMode.type === 'paginated' ? editor.displayZoom : 1;
    const anchorLeft = pageRect.left - containerRect.left + endpoint.bounds.x * zoom;
    const anchorTop = pageRect.top - containerRect.top + endpoint.bounds.y * zoom;

    const stemHeight = endpoint.bounds.height * zoom;
    const totalHeight = HANDLE_RADIUS * 2 + stemHeight;
    const touchHeight = Math.max(totalHeight, TOUCH_TARGET_SIZE);

    const customPaintTop = type === 'from' ? -(HANDLE_RADIUS * 2) : 0;
    const handleCenterY = customPaintTop + totalHeight / 2;
    const touchTargetTop = handleCenterY - touchHeight / 2;

    const handleXOffset = (type === 'from' ? -STEM_WIDTH : STEM_WIDTH) / 2;
    const touchTargetLeft = handleXOffset - TOUCH_TARGET_SIZE / 2;

    const paintTop = customPaintTop - touchTargetTop;
    const paintLeft = (TOUCH_TARGET_SIZE - HANDLE_RADIUS * 2) / 2;

    return {
      left: anchorLeft + touchTargetLeft,
      top: anchorTop + touchTargetTop,
      touchHeight,
      paintLeft,
      paintTop,
      stemHeight,
    };
  }

  const handleStyle = css({
    position: 'absolute',
    zIndex: 'menu',
    pointerEvents: 'auto',
    touchAction: 'none',
    background: 'transparent',
    border: 'none',
    padding: '0',
    margin: '0',
    WebkitTapHighlightColor: 'transparent',
  });

  const paintStyle = css({
    position: 'absolute',
    width: `${HANDLE_RADIUS * 2}px`,
  });

  const stemStyle = css({
    position: 'absolute',
    left: `${HANDLE_RADIUS - STEM_WIDTH / 2}px`,
    width: `${STEM_WIDTH}px`,
    backgroundColor: 'text.default',
    borderRadius: 'full',
  });

  const circleStyle = css({
    position: 'absolute',
    width: `${HANDLE_RADIUS * 2}px`,
    height: `${HANDLE_RADIUS * 2}px`,
    borderRadius: 'full',
    backgroundColor: 'text.default',
  });

  function handlePointerDown(type: SelectionHandleKind, event: PointerEvent) {
    if (!isTouchLikePointer(event)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId)) {
      target.setPointerCapture(event.pointerId);
    }

    editor.touchGesture.handleSelectionHandlePointerDown(type, event);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isTouchLikePointer(event)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    editor.touchGesture.handleSelectionHandlePointerMove(event);
  }

  function handlePointerUp(event: PointerEvent) {
    if (!isTouchLikePointer(event)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }

    editor.touchGesture.handleSelectionHandlePointerUp(event);
  }
</script>

{#if fromHandle}
  <button
    style:left={`${fromHandle.left}px`}
    style:top={`${fromHandle.top}px`}
    style:width={`${TOUCH_TARGET_SIZE}px`}
    style:height={`${fromHandle.touchHeight}px`}
    class={handleStyle}
    aria-label="Selection start handle"
    data-pointer-capture
    onpointercancel={handlePointerUp}
    onpointerdown={(event) => handlePointerDown('from', event)}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    type="button"
  >
    <div
      style:left={`${fromHandle.paintLeft}px`}
      style:top={`${fromHandle.paintTop}px`}
      style:height={`${HANDLE_RADIUS * 2 + fromHandle.stemHeight}px`}
      class={paintStyle}
    >
      <div style:top="0" class={circleStyle}></div>
      <div style:top={`${HANDLE_RADIUS * 2}px`} style:height={`${fromHandle.stemHeight}px`} class={stemStyle}></div>
    </div>
  </button>
{/if}

{#if toHandle}
  <button
    style:left={`${toHandle.left}px`}
    style:top={`${toHandle.top}px`}
    style:width={`${TOUCH_TARGET_SIZE}px`}
    style:height={`${toHandle.touchHeight}px`}
    class={handleStyle}
    aria-label="Selection end handle"
    data-pointer-capture
    onpointercancel={handlePointerUp}
    onpointerdown={(event) => handlePointerDown('to', event)}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    type="button"
  >
    <div
      style:left={`${toHandle.paintLeft}px`}
      style:top={`${toHandle.paintTop}px`}
      style:height={`${HANDLE_RADIUS * 2 + toHandle.stemHeight}px`}
      class={paintStyle}
    >
      <div style:top="0" style:height={`${toHandle.stemHeight}px`} class={stemStyle}></div>
      <div style:top={`${toHandle.stemHeight}px`} class={circleStyle}></div>
    </div>
  </button>
{/if}
