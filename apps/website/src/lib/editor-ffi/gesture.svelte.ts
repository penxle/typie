import {
  LONG_PRESS_CANCEL_DISTANCE_PX,
  LONG_PRESS_MS,
  NATIVE_CONTEXTMENU_SUPPRESS_AFTER_LONGPRESS_MS,
  TOUCH_MENU_ESTIMATED_HEIGHT,
  TOUCH_MENU_VIEWPORT_PADDING,
} from './constants';
import { EditorEdgeAutoScroll } from './edge-auto-scroll';
import { tryHandleInteractiveHit } from './handlers/pointer';
import type { InputModifiers, PageRect, Position, Selection, SelectionEndpoints } from '@typie/editor-ffi/browser';
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
  endpoint: PageRect;
  pageRect: DOMRect;
  surfaceRect: DOMRect;
  zoom: number;
};

export const computeSelectionHandleVisual = ({
  kind,
  endpoint,
  pageRect,
  surfaceRect,
  zoom,
}: SelectionHandleVisualInput): SelectionHandleVisual => {
  const radius = SELECTION_HANDLE_RADIUS;
  const stemWidth = SELECTION_HANDLE_STEM_WIDTH;
  const touchTargetSize = SELECTION_HANDLE_TOUCH_TARGET_SIZE;

  const anchorLeft = pageRect.left - surfaceRect.left + endpoint.rect.x * zoom;
  const anchorTop = pageRect.top - surfaceRect.top + endpoint.rect.y * zoom;

  const stemHeight = endpoint.rect.height * zoom;
  const totalHeight = radius * 2 + stemHeight;
  const touchHeight = Math.max(totalHeight, touchTargetSize);

  const customPaintTop = kind === 'from' ? -(radius * 2) : 0;
  const handleCenterY = customPaintTop + totalHeight / 2;
  const touchTargetTop = handleCenterY - touchHeight / 2;

  const handleXOffset = kind === 'from' ? -stemWidth / 2 : stemWidth / 2;
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

type Phase = 'idle' | 'pressing' | 'tapMoved' | 'longPressed' | 'handleDragging';

type ResolvedTouchPoint = {
  page: number;
  x: number;
  y: number;
};

type DeferredDown = {
  page: number;
  x: number;
  y: number;
  count: number;
  modifiers: InputModifiers;
};

const ZERO_MODIFIERS: InputModifiers = { shift: false, ctrl: false, alt: false, meta: false };

const distance = (a: { x: number; y: number }, b: { x: number; y: number }): number => Math.hypot(a.x - b.x, a.y - b.y);

export class TouchGestureController {
  #editor: Editor;
  #phase: Phase = $state('idle');
  #pressGeneration = 0;
  #activePointerId: number | null = null;
  #pressStart: { x: number; y: number; time: number } | null = null;
  #lastClientPoint: { x: number; y: number } | null = null;
  #pendingDown: DeferredDown | null = null;
  #dragAnchor: Position | null = null;
  #baseSelection: Selection | undefined;
  #selectionHit = false;
  #longPressTimer: ReturnType<typeof setTimeout> | null = null;
  #suppressNativeContextMenuUntil = 0;
  #edgeAutoScroll = new EditorEdgeAutoScroll();

  constructor(editor: Editor) {
    this.#editor = editor;
  }

  get gestureActive(): boolean {
    return this.#editor.readOnly && this.#phase !== 'idle';
  }

  shouldSuppressNativeContextMenu(): boolean {
    return performance.now() < this.#suppressNativeContextMenuUntil;
  }

  handlePointerDown(e: PointerEvent, resolved: ResolvedTouchPoint | null): void {
    if (!e.isPrimary) return;

    this.#pressGeneration++;
    const generation = this.#pressGeneration;

    this.#activePointerId = e.pointerId;
    this.#pressStart = { x: e.clientX, y: e.clientY, time: performance.now() };
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#selectionHit = resolved ? this.#editor.selectionHitTest(resolved.page, resolved.x, resolved.y) : false;
    this.#pendingDown = resolved ? { page: resolved.page, x: resolved.x, y: resolved.y, count: 1, modifiers: ZERO_MODIFIERS } : null;
    this.#phase = 'pressing';
    this.#suppressNativeContextMenuUntil = performance.now() + LONG_PRESS_MS + NATIVE_CONTEXTMENU_SUPPRESS_AFTER_LONGPRESS_MS;
    this.#clearLongPressTimer();
    this.#longPressTimer = setTimeout(() => {
      this.#onLongPressFire(generation);
    }, LONG_PRESS_MS);
  }

  handlePointerMove(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) return;
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };

    if (this.#phase === 'pressing') {
      if (this.#pressStart && distance(this.#pressStart, { x: e.clientX, y: e.clientY }) > LONG_PRESS_CANCEL_DISTANCE_PX) {
        this.#clearLongPressTimer();
        const downEnqueued = this.#flushDeferredDown();
        if (!downEnqueued) {
          this.#reset();
          return;
        }
        this.#phase = 'tapMoved';
        this.#routeMoveToWasm(e);
        this.#updateEdgeAutoScroll();
      }
      return;
    }

    if (this.#phase === 'tapMoved') {
      this.#routeMoveToWasm(e);
      this.#updateEdgeAutoScroll();
      return;
    }
  }

  handlePointerUp(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) return;

    switch (this.#phase) {
      case 'pressing': {
        this.#clearLongPressTimer();
        this.#flushDeferredDown();
        break;
      }
      case 'tapMoved': {
        break;
      }
      case 'longPressed': {
        break;
      }
      case 'idle': {
        break;
      }
    }
    this.#reset();
  }

  handlePointerCancel(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) return;

    this.#reset();
  }

  handleSelectionHandlePointerDown(type: SelectionHandleKind, e: PointerEvent): void {
    const endpoints = this.#editor.selectionEndpoints();
    if (!endpoints) return;

    this.#activePointerId = e.pointerId;
    this.#phase = 'handleDragging';
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#dragAnchor = type === 'from' ? endpoints.to_position : endpoints.from_position;
    this.#baseSelection = undefined;
    this.#pendingDown = null;
    this.#selectionHit = false;
    this.#clearLongPressTimer();
    this.#editor.closeContextMenu();
    this.#routeMoveToClientPoint(e.clientX, e.clientY);
    this.#editor.flush();
  }

  handleSelectionHandlePointerMove(e: PointerEvent): void {
    if (this.#phase !== 'handleDragging' || this.#activePointerId !== e.pointerId) return;

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    if (this.#routeMoveToClientPoint(e.clientX, e.clientY)) {
      this.#editor.flush();
    }
    this.#updateEdgeAutoScroll();
  }

  handleSelectionHandlePointerUp(e: PointerEvent): void {
    if (this.#phase !== 'handleDragging' || this.#activePointerId !== e.pointerId) return;

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    if (this.#routeMoveToClientPoint(e.clientX, e.clientY)) {
      this.#editor.flush();
    }
    this.#requestTouchMenuOpen(this.#pressGeneration, this.#lastClientPoint);
    this.#reset();
  }

  destroy(): void {
    this.#clearLongPressTimer();
    this.#reset();
  }

  #onLongPressFire(generation: number): void {
    if (this.#phase !== 'pressing' || generation !== this.#pressGeneration) return;
    this.#phase = 'longPressed';

    if (this.#selectionHit) {
      this.#pendingDown = null;
      this.#requestTouchMenuOpen(generation);
      return;
    }

    const down = this.#pendingDown;
    this.#pendingDown = null;
    if (!down) {
      this.#requestTouchMenuOpen(generation);
      return;
    }

    this.#editor.enqueue({
      type: 'selection',
      op: { type: 'select_unit_at', page: down.page, x: down.x, y: down.y, unit: 'word' },
    });
    this.#editor.flush();

    const fallbackPoint = this.#lastClientPoint ? { x: this.#lastClientPoint.x, y: this.#lastClientPoint.y } : null;

    requestAnimationFrame(() => {
      this.#requestTouchMenuOpen(generation, fallbackPoint);
    });
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

  #flushDeferredDown(): boolean {
    const down = this.#pendingDown;
    if (!down) return false;
    this.#pendingDown = null;

    const hit = this.#editor.interactiveHitTest(down.page, down.x, down.y);
    if (hit && tryHandleInteractiveHit(this.#editor, hit, { x: down.x, y: down.y })) {
      return false;
    }
    this.#editor.enqueue({
      type: 'selection',
      op: { type: 'set_at', page: down.page, x: down.x, y: down.y },
    });
    this.#editor.flush();
    const selection = this.#editor.selection;
    this.#dragAnchor = selection?.anchor ?? null;
    this.#baseSelection = undefined;
    return true;
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
    if ((this.#phase !== 'tapMoved' && this.#phase !== 'handleDragging') || !this.#lastClientPoint) {
      this.#edgeAutoScroll.stop();
      return;
    }

    this.#edgeAutoScroll.update(
      this.#editor,
      { clientX: this.#lastClientPoint.x, clientY: this.#lastClientPoint.y },
      (clientX, clientY) => {
        this.#lastClientPoint = { x: clientX, y: clientY };
        if ((this.#phase === 'tapMoved' || this.#phase === 'handleDragging') && this.#routeMoveToClientPoint(clientX, clientY)) {
          this.#editor.flush();
        }
      },
    );
  }

  #clearLongPressTimer(): void {
    if (this.#longPressTimer !== null) {
      clearTimeout(this.#longPressTimer);
      this.#longPressTimer = null;
    }
  }

  #reset(): void {
    this.#clearLongPressTimer();
    this.#phase = 'idle';
    this.#activePointerId = null;
    this.#pressStart = null;
    this.#lastClientPoint = null;
    this.#pendingDown = null;
    this.#dragAnchor = null;
    this.#baseSelection = undefined;
    this.#selectionHit = false;
    this.#edgeAutoScroll.stop();
  }
}
