import {
  DOUBLE_TAP_DISTANCE_PX,
  DOUBLE_TAP_INTERVAL_MS,
  LONG_PRESS_CANCEL_DISTANCE_PX,
  LONG_PRESS_MS,
  NATIVE_CONTEXTMENU_SUPPRESS_AFTER_LONGPRESS_MS,
  TOUCH_DRAG_START_DISTANCE_PX,
  TOUCH_MENU_ESTIMATED_HEIGHT,
  TOUCH_MENU_VIEWPORT_PADDING,
} from './constants';
import { EditorEdgeAutoScroll } from './edge-auto-scroll';
import { tryHandleInteractiveHit } from './handlers/pointer';
import type { Position, Selection, SelectionEndpoints } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

export type SelectionHandleKind = 'from' | 'to';

export type TouchMenuPosition = {
  x: number;
  y: number;
  placement: 'top' | 'bottom';
};

export type TouchMenuPositionInput = {
  endpoints: SelectionEndpoints;
  pageRects: (DOMRect | undefined)[];
  zoom: number;
  viewport: { left: number; top: number; width: number; height: number };
};

const clamp = (value: number, min: number, max: number): number => Math.min(Math.max(value, min), max);

export const computeTouchContextMenuPosition = ({
  endpoints,
  pageRects,
  zoom,
  viewport,
}: TouchMenuPositionInput): TouchMenuPosition | null => {
  const fromPageRect = pageRects[endpoints.from.page_idx];
  const toPageRect = pageRects[endpoints.to.page_idx];
  if (!fromPageRect || !toPageRect) return null;

  const fromLeft = fromPageRect.left + endpoints.from.rect.x * zoom;
  const fromRight = fromLeft + endpoints.from.rect.width * zoom;
  const fromTop = fromPageRect.top + endpoints.from.rect.y * zoom;
  const fromBottom = fromTop + endpoints.from.rect.height * zoom;

  const toLeft = toPageRect.left + endpoints.to.rect.x * zoom;
  const toRight = toLeft + endpoints.to.rect.width * zoom;
  const toTop = toPageRect.top + endpoints.to.rect.y * zoom;
  const toBottom = toTop + endpoints.to.rect.height * zoom;

  const selLeft = Math.min(fromLeft, toLeft);
  const selRight = Math.max(fromRight, toRight);
  const selTop = Math.min(fromTop, toTop);
  const selBottom = Math.max(fromBottom, toBottom);

  const viewportLeft = viewport.left;
  const viewportTop = viewport.top;
  const viewportRight = viewport.left + viewport.width;
  const viewportBottom = viewport.top + viewport.height;

  const spaceAbove = selTop - viewportTop;
  const spaceBelow = viewportBottom - selBottom;

  const placement: 'top' | 'bottom' = spaceAbove >= TOUCH_MENU_ESTIMATED_HEIGHT || spaceAbove >= spaceBelow ? 'top' : 'bottom';
  const anchorY = placement === 'top' ? selTop : selBottom;
  const centerX = (selLeft + selRight) / 2;

  return {
    x: clamp(centerX, viewportLeft + TOUCH_MENU_VIEWPORT_PADDING, viewportRight - TOUCH_MENU_VIEWPORT_PADDING),
    y: clamp(anchorY, viewportTop + TOUCH_MENU_VIEWPORT_PADDING, viewportBottom - TOUCH_MENU_VIEWPORT_PADDING),
    placement,
  };
};

export const SELECTION_HANDLE_RADIUS = 8;
export const SELECTION_HANDLE_STEM_WIDTH = 2;
export const SELECTION_HANDLE_TOUCH_TARGET_SIZE = 44;

export type SelectionHandleVisual = {
  left: number;
  top: number;
  touchHeight: number;
  paintLeft: number;
  paintTop: number;
  stemHeight: number;
};

export type SelectionHandleVisualInput = {
  kind: SelectionHandleKind;
  anchorRect: DOMRect;
};

export const computeSelectionHandleVisual = ({ kind, anchorRect }: SelectionHandleVisualInput): SelectionHandleVisual => {
  const radius = SELECTION_HANDLE_RADIUS;
  const stemWidth = SELECTION_HANDLE_STEM_WIDTH;
  const touchTargetSize = SELECTION_HANDLE_TOUCH_TARGET_SIZE;

  const anchorLeft = anchorRect.left;
  const anchorTop = anchorRect.top;
  const stemHeight = anchorRect.height;
  const totalHeight = radius * 2 + stemHeight;
  const touchHeight = Math.max(totalHeight, touchTargetSize);

  const customPaintTop = kind === 'from' ? -(radius * 2) : 0;
  const handleCenterY = customPaintTop + totalHeight / 2;
  const touchTargetTop = handleCenterY - touchHeight / 2;

  const handleXOffset = (kind === 'from' ? -stemWidth : stemWidth) / 2;
  const touchTargetLeft = handleXOffset - touchTargetSize / 2;

  const paintTop = customPaintTop - touchTargetTop;
  const paintLeft = (touchTargetSize - radius * 2) / 2;

  return {
    left: anchorLeft + touchTargetLeft,
    top: anchorTop + touchTargetTop,
    touchHeight,
    paintLeft,
    paintTop,
    stemHeight,
  };
};

type Phase = 'idle' | 'pressing' | 'doubleTapPending' | 'doubleTapDragging' | 'handleDragging' | 'dndArmed';

type ResolvedTouchPoint = {
  page: number;
  x: number;
  y: number;
};

type PressRecord = {
  time: number;
  x: number;
  y: number;
  selectionHit: boolean;
};

type TapRecord = {
  time: number;
  x: number;
  y: number;
};

const distance = (a: { x: number; y: number }, b: { x: number; y: number }): number => Math.hypot(a.x - b.x, a.y - b.y);

export class TouchGestureController {
  #editor: Editor;
  #phase: Phase = $state('idle');
  #pressGeneration = 0;
  #activePointerId: number | null = $state(null);
  #press: PressRecord | null = null;
  #lastTap: TapRecord | null = null;
  #lastClientPoint: { x: number; y: number } | null = null;
  #doubleTapStart: { x: number; y: number } | null = null;
  #pendingSelectionHandleType: SelectionHandleKind | null = null;
  #dragAnchor: Position | null = null;
  #baseSelection: Selection | undefined;
  #suppressTapOnPointerUp = false;
  #movedPastTapThreshold = false;
  #readOnlyDragStarted = false;
  #dragArmed = false;
  #dragCandidate = false;
  #wasTouchMenuOpenOnPointerDown = false;
  #longPressTimer: ReturnType<typeof setTimeout> | null = null;
  #suppressNativeContextMenuUntil = 0;
  #edgeAutoScroll = new EditorEdgeAutoScroll();

  constructor(editor: Editor) {
    this.#editor = editor;
  }

  #selectWordAt(point: ResolvedTouchPoint): void {
    this.#editor.enqueue({
      type: 'selection',
      op: { type: 'select_unit_at', page: point.page, x: point.x, y: point.y, unit: 'word' },
    });
    this.#editor.flush();

    const selection = this.#editor.selection;
    if (selection && !this.#editor.isSelectionCollapsed) {
      this.#dragAnchor = selection.anchor;
      this.#baseSelection = selection;
    } else {
      this.#dragAnchor = null;
      this.#baseSelection = undefined;
    }
  }

  #onLongPressFire(generation: number): void {
    if (this.#phase !== 'pressing' || generation !== this.#pressGeneration || !this.#press) return;

    const point = this.#lastClientPoint ?? this.#press;
    const local = this.#editor.clientToLocal(point.x, point.y);
    if (!local) return;

    if (this.#press.selectionHit) {
      this.#phase = 'dndArmed';
      this.#dragCandidate = true;
      this.#dragArmed = true;
      this.#suppressTapOnPointerUp = true;
      this.#updateEdgeAutoScroll();
      return;
    }

    this.#selectWordAt(local);
    this.#phase = 'doubleTapPending';
    this.#doubleTapStart = { x: point.x, y: point.y };
    this.#suppressTapOnPointerUp = true;
  }

  #requestTouchMenuOpen(generation: number, fallbackPoint: { x: number; y: number } | null = this.#lastClientPoint): void {
    if (generation !== this.#pressGeneration) return;

    const extraItems = this.#collectTouchContextMenuItems(fallbackPoint);

    const endpoints = this.#editor.selectionEndpoints();
    if (!endpoints) {
      this.#openTouchMenuAtFallback(fallbackPoint, extraItems);
      return;
    }

    const pageSizes = this.#editor.pageSizes ?? [];
    const pageRects: (DOMRect | undefined)[] = Array.from({ length: pageSizes.length });
    for (let i = 0; i < pageSizes.length; i++) {
      const el = this.#editor.pageEls[i];
      pageRects[i] = el ? el.getBoundingClientRect() : undefined;
    }

    const visualViewport = typeof window === 'undefined' ? null : window.visualViewport;
    const viewport = {
      left: visualViewport?.offsetLeft ?? 0,
      top: visualViewport?.offsetTop ?? 0,
      width: visualViewport?.width ?? (typeof window === 'undefined' ? 0 : window.innerWidth),
      height: visualViewport?.height ?? (typeof window === 'undefined' ? 0 : window.innerHeight),
    };

    const position = computeTouchContextMenuPosition({ endpoints, pageRects, zoom: this.#editor.safeDisplayZoom(), viewport });
    if (!position) {
      this.#openTouchMenuAtFallback(fallbackPoint, extraItems);
      return;
    }

    this.#editor.openContextMenu({
      x: position.x,
      y: position.y,
      source: 'touch',
      placement: position.placement,
      extraItems,
    });
  }

  #openTouchMenuAtFallback(
    point: { x: number; y: number } | null,
    extraItems: ReturnType<Editor['collectContextMenuContributions']>,
  ): void {
    if (!point) return;
    this.#editor.openContextMenu({ x: point.x, y: point.y, source: 'touch', placement: 'bottom', extraItems });
  }

  #collectTouchContextMenuItems(point: { x: number; y: number } | null): ReturnType<Editor['collectContextMenuContributions']> {
    if (!point) return [];

    const local = this.#editor.clientToLocal(point.x, point.y);
    const hit = local ? this.#editor.interactiveHitTest(local.page, local.x, local.y) : undefined;
    return this.#editor.collectContextMenuContributions({
      hit,
      clientX: point.x,
      clientY: point.y,
    });
  }

  #routeMoveToWasm(e: PointerEvent): boolean {
    return this.#routeMoveToClientPoint(e.clientX, e.clientY);
  }

  #routeMoveToClientPoint(clientX: number, clientY: number): boolean {
    const local = this.#editor.clientToLocal(clientX, clientY);
    if (!local || !this.#dragAnchor) return false;
    this.#editor.enqueue({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: this.#dragAnchor,
        head_page: local.page,
        head_x: local.x,
        head_y: local.y,
        base_selection: this.#baseSelection,
        allow_collapse: this.#phase !== 'handleDragging' && this.#baseSelection === undefined,
      },
    });
    return true;
  }

  #updateEdgeAutoScroll(): void {
    if (!this.#shouldAutoScroll() || !this.#lastClientPoint) {
      this.#edgeAutoScroll.stop();
      return;
    }

    this.#edgeAutoScroll.update(
      this.#editor,
      { clientX: this.#lastClientPoint.x, clientY: this.#lastClientPoint.y },
      (clientX, clientY) => {
        this.#lastClientPoint = { x: clientX, y: clientY };
        if ((this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging') && this.#routeMoveToClientPoint(clientX, clientY)) {
          this.#editor.flush();
        }
      },
    );
  }

  #shouldAutoScroll(): boolean {
    return this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging' || this.#phase === 'dndArmed';
  }

  #clearLongPressTimer(): void {
    if (this.#longPressTimer === null) {
      return;
    }

    clearTimeout(this.#longPressTimer);
    this.#longPressTimer = null;
  }

  #clearNativeSelection(): void {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      window.getSelection()?.removeAllRanges();
    } catch {
      // ignore
    }
  }

  #resetSession(): void {
    this.#clearLongPressTimer();
    this.#phase = 'idle';
    this.#activePointerId = null;
    this.#press = null;
    this.#lastClientPoint = null;
    this.#doubleTapStart = null;
    this.#pendingSelectionHandleType = null;
    this.#dragAnchor = null;
    this.#baseSelection = undefined;
    this.#suppressTapOnPointerUp = false;
    this.#movedPastTapThreshold = false;
    this.#readOnlyDragStarted = false;
    this.#dragArmed = false;
    this.#dragCandidate = false;
    this.#wasTouchMenuOpenOnPointerDown = false;
    this.#edgeAutoScroll.stop();
  }

  get gestureActive(): boolean {
    return this.#editor.readOnly && (this.#activePointerId !== null || this.#phase !== 'idle');
  }

  get panLockActive(): boolean {
    if (!this.#editor.readOnly) {
      return false;
    }

    return (
      this.#phase === 'doubleTapPending' ||
      this.#phase === 'doubleTapDragging' ||
      this.#phase === 'handleDragging' ||
      this.#phase === 'dndArmed'
    );
  }

  get isDoubleTapSelectionDragActive(): boolean {
    return this.#phase === 'doubleTapPending' || this.#phase === 'doubleTapDragging';
  }

  shouldSuppressNativeContextMenu(): boolean {
    return performance.now() < this.#suppressNativeContextMenuUntil;
  }

  isReadOnlyTouchDragArmed(): boolean {
    return this.#dragArmed;
  }

  isReadOnlyTouchDragCandidate(): boolean {
    return this.#dragCandidate;
  }

  handlePointerDown(e: PointerEvent, resolved: ResolvedTouchPoint | null, selectionHandleType: SelectionHandleKind | null = null): void {
    if (!e.isPrimary) return;

    this.#clearNativeSelection();

    if (this.#activePointerId !== null && this.#activePointerId !== e.pointerId) {
      this.cancelSession();
      return;
    }

    this.#pressGeneration++;
    const generation = this.#pressGeneration;
    const selectionHandleEndpoints = selectionHandleType ? this.#editor.selectionEndpoints() : null;

    this.#activePointerId = e.pointerId;
    this.#press = {
      time: performance.now(),
      x: e.clientX,
      y: e.clientY,
      selectionHit: resolved ? this.#editor.selectionHitTest(resolved.page, resolved.x, resolved.y) : false,
    };
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#doubleTapStart = null;
    this.#pendingSelectionHandleType = selectionHandleEndpoints ? selectionHandleType : null;
    this.#dragAnchor =
      selectionHandleType === 'from' ? (selectionHandleEndpoints?.to_position ?? null) : (selectionHandleEndpoints?.from_position ?? null);
    this.#baseSelection = undefined;
    this.#suppressTapOnPointerUp = false;
    this.#movedPastTapThreshold = false;
    this.#readOnlyDragStarted = false;
    this.#dragArmed = false;
    this.#dragCandidate = false;
    this.#edgeAutoScroll.stop();
    this.#clearLongPressTimer();
    this.#phase = 'pressing';
    this.#suppressNativeContextMenuUntil = performance.now() + LONG_PRESS_MS + NATIVE_CONTEXTMENU_SUPPRESS_AFTER_LONGPRESS_MS;
    this.#wasTouchMenuOpenOnPointerDown = this.#editor.contextMenu.isOpen && this.#editor.contextMenu.source === 'touch';
    if (this.#wasTouchMenuOpenOnPointerDown) {
      this.#editor.closeContextMenu();
    }

    if (this.#pendingSelectionHandleType) {
      this.#lastTap = null;
      this.#editor.closeContextMenu();
      return;
    }

    const isDoubleTap =
      this.#lastTap !== null &&
      this.#press.time - this.#lastTap.time <= DOUBLE_TAP_INTERVAL_MS &&
      distance(this.#lastTap, this.#press) <= DOUBLE_TAP_DISTANCE_PX;

    if (isDoubleTap && resolved) {
      this.#selectWordAt(resolved);
      this.#phase = 'doubleTapPending';
      this.#doubleTapStart = { x: e.clientX, y: e.clientY };
      this.#suppressTapOnPointerUp = true;
      this.#lastTap = null;
      return;
    }

    this.#longPressTimer = setTimeout(() => {
      this.#onLongPressFire(generation);
    }, LONG_PRESS_MS);
  }

  handlePointerMove(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) return;
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };

    if (this.#phase === 'pressing') {
      if (this.#pendingSelectionHandleType && this.#press) {
        if (distance(this.#press, this.#lastClientPoint) < TOUCH_DRAG_START_DISTANCE_PX) {
          return;
        }

        this.#movedPastTapThreshold = true;
        this.#clearLongPressTimer();
        this.#phase = 'handleDragging';
        if (this.#routeMoveToWasm(e)) {
          this.#editor.flush();
        }
        this.#updateEdgeAutoScroll();
        e.preventDefault();
        return;
      }

      if (this.#press && distance(this.#press, this.#lastClientPoint) > LONG_PRESS_CANCEL_DISTANCE_PX) {
        this.#movedPastTapThreshold = true;
        this.#clearLongPressTimer();
        this.#dragCandidate = false;
      }
      return;
    }

    if (this.#phase === 'doubleTapPending') {
      e.preventDefault();
      const start = this.#doubleTapStart;
      if (!start) return;

      if (distance(start, this.#lastClientPoint) < TOUCH_DRAG_START_DISTANCE_PX) {
        return;
      }

      this.#phase = 'doubleTapDragging';
    }

    if (this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging') {
      if (this.#routeMoveToWasm(e)) {
        this.#editor.flush();
      }
      this.#updateEdgeAutoScroll();
      e.preventDefault();
      return;
    }

    if (this.#phase === 'dndArmed') {
      this.#updateEdgeAutoScroll();
    }
  }

  handlePointerUp(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) return;

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#clearLongPressTimer();
    this.#edgeAutoScroll.stop();

    switch (this.#phase) {
      case 'doubleTapDragging':
      case 'handleDragging': {
        if (this.#routeMoveToWasm(e)) {
          this.#editor.flush();
        }
        this.#requestTouchMenuOpen(this.#pressGeneration, this.#lastClientPoint);
        this.#lastTap = null;
        break;
      }
      case 'doubleTapPending': {
        this.#requestTouchMenuOpen(this.#pressGeneration, this.#lastClientPoint);
        this.#lastTap = null;
        break;
      }
      case 'dndArmed': {
        if (!this.#readOnlyDragStarted) {
          this.#requestTouchMenuOpen(this.#pressGeneration, this.#lastClientPoint);
        }
        this.#lastTap = null;
        break;
      }
      case 'pressing': {
        if (!this.#suppressTapOnPointerUp && !this.#movedPastTapThreshold && this.#press) {
          const local = this.#editor.clientToLocal(e.clientX, e.clientY);
          if (local) {
            const hit = this.#editor.interactiveHitTest(local.page, local.x, local.y);
            if (hit && tryHandleInteractiveHit(this.#editor, hit, { x: local.x, y: local.y })) {
              this.#editor.flush();
              this.#editor.closeContextMenu();
              this.#lastTap = null;
              break;
            }

            if (this.#editor.selectionHitTest(local.page, local.x, local.y)) {
              if (!this.#wasTouchMenuOpenOnPointerDown) {
                this.#requestTouchMenuOpen(this.#pressGeneration, this.#lastClientPoint);
              }
            } else {
              this.#editor.enqueue({
                type: 'selection',
                op: { type: 'set_at', page: local.page, x: local.x, y: local.y },
              });
              this.#editor.flush();
              this.#editor.closeContextMenu();
            }
            this.#lastTap = { time: performance.now(), x: e.clientX, y: e.clientY };
          } else {
            this.#editor.closeContextMenu();
            this.#lastTap = null;
          }
        } else {
          this.#lastTap = null;
        }
        break;
      }
      case 'idle': {
        break;
      }
    }
    this.#resetSession();
  }

  handlePointerCancel(e: PointerEvent): void {
    if (this.#activePointerId !== null && this.#activePointerId !== e.pointerId) return;

    this.cancelSession();
  }

  handleNativeDragStart(): void {
    if (this.#phase !== 'dndArmed') {
      return;
    }

    this.#readOnlyDragStarted = true;
    this.#editor.closeContextMenu();
    this.#clearLongPressTimer();
    this.#edgeAutoScroll.stop();
  }

  handleNativeDragEnd(): void {
    this.#readOnlyDragStarted = false;
    this.#resetSession();
  }

  resetReadOnlyTouchState(): void {
    this.cancelSession();
  }

  cancelSession(): void {
    this.#lastTap = null;
    this.#resetSession();
  }

  handleSelectionHandlePointerDown(type: SelectionHandleKind, e: PointerEvent): void {
    const local = this.#editor.clientToLocal(e.clientX, e.clientY);
    const resolved = local ? { page: local.page, x: local.x, y: local.y } : null;
    this.handlePointerDown(e, resolved, type);
  }

  handleSelectionHandlePointerMove(e: PointerEvent): void {
    this.handlePointerMove(e);
  }

  handleSelectionHandlePointerUp(e: PointerEvent): void {
    this.handlePointerUp(e);
  }

  destroy(): void {
    this.cancelSession();
  }
}
