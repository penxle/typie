import {
  LONG_PRESS_CANCEL_DISTANCE_PX,
  LONG_PRESS_MS,
  NATIVE_CONTEXTMENU_SUPPRESS_AFTER_LONGPRESS_MS,
  TOUCH_MENU_ESTIMATED_HEIGHT,
  TOUCH_MENU_VIEWPORT_PADDING,
} from './constants';
import { tryHandleInteractiveHit } from './handlers/pointer';
import type { InputModifiers, Position, Selection, SelectionEndpoints } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

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

type Phase = 'idle' | 'pressing' | 'tapMoved' | 'longPressed';

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
  #phase: Phase = 'idle';
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

  constructor(editor: Editor) {
    this.#editor = editor;
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
      }
      return;
    }

    if (this.#phase === 'tapMoved') {
      this.#routeMoveToWasm(e);
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

    const endpoints = this.#editor.selectionEndpoints();
    if (!endpoints) {
      this.#openTouchMenuAtFallback(fallbackPoint);
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
      this.#openTouchMenuAtFallback(fallbackPoint);
      return;
    }

    this.#editor.openContextMenu({
      x: position.x,
      y: position.y,
      source: 'touch',
      placement: position.placement,
    });
  }

  #openTouchMenuAtFallback(point: { x: number; y: number } | null): void {
    if (!point) return;
    this.#editor.openContextMenu({ x: point.x, y: point.y, source: 'touch', placement: 'bottom' });
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

  #routeMoveToWasm(e: PointerEvent): void {
    const local = this.#editor.clientToLocal(e.clientX, e.clientY);
    if (!local || !this.#dragAnchor) return;
    this.#editor.enqueue({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: this.#dragAnchor,
        head_page: local.page,
        head_x: local.x,
        head_y: local.y,
        base_selection: this.#baseSelection,
      },
    });
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
  }
}
