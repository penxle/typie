<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import {
    computeSelectionHandleVisual,
    SELECTION_HANDLE_RADIUS as HANDLE_RADIUS,
    SELECTION_HANDLE_STEM_WIDTH as STEM_WIDTH,
    SELECTION_HANDLE_TOUCH_TARGET_SIZE as TOUCH_TARGET_SIZE,
  } from '$lib/editor-ffi/gesture.svelte';
  import { pageRectToClientRect } from '../geometry';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';
  import type { SelectionHandleKind, SelectionHandleVisual } from '$lib/editor-ffi/gesture.svelte';

  const { editor } = getEditorContext();
  const viewportOverlay = getViewportOverlayContext();

  const fromHandle = $derived.by(() => getHandleVisual('from'));
  const toHandle = $derived.by(() => getHandleVisual('to'));

  function getHandleVisual(type: SelectionHandleKind): SelectionHandleVisual | null {
    if (!editor) {
      return null;
    }

    void viewportOverlay.change;
    void editor.displayZoom;
    void editor.pageSizes;
    const selection = editor.selection;

    if (!editor.readOnly || !isTouchCapable() || !selection) {
      return null;
    }

    const endpoints = editor.selectionEndpoints();
    if (!endpoints) {
      return null;
    }

    const endpoint = type === 'from' ? endpoints.from : endpoints.to;
    const anchorRect = pageRectToClientRect(editor, endpoint);
    if (!anchorRect) {
      return null;
    }

    return computeSelectionHandleVisual({
      kind: type,
      anchorRect,
    });
  }

  function isTouchLikePointer(event: PointerEvent): boolean {
    return event.pointerType === 'touch';
  }

  function isTouchCapable(): boolean {
    return typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0;
  }

  const handleStyle = css({
    position: 'fixed',
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

    editor?.gesture.handleSelectionHandlePointerDown(type, event);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isTouchLikePointer(event)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    editor?.gesture.handleSelectionHandlePointerMove(event);
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

    editor?.gesture.handleSelectionHandlePointerUp(event);
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
