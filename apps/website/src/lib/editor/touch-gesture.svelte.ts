import { handleDragScroll } from '@typie/ui/utils';
import {
  TOUCH_DOUBLE_TAP_DISTANCE_PX,
  TOUCH_DOUBLE_TAP_INTERVAL_MS,
  TOUCH_DRAG_START_DISTANCE_PX,
  TOUCH_EDGE_MAX_SCROLL_SPEED,
  TOUCH_EDGE_MIN_SCROLL_SPEED,
  TOUCH_EDGE_SCROLL_THRESHOLD_PX,
  TOUCH_LONG_PRESS_CANCEL_DISTANCE_PX,
  TOUCH_LONG_PRESS_MS,
} from './constants';
import type { Placement } from '@floating-ui/dom';
import type { Editor } from './editor.svelte';
import type { Position, Selection, SelectionEndpointBounds } from './types';

export type SelectionHandleKind = 'from' | 'to';

export type OrderedSelectionHandles = {
  from: SelectionEndpointBounds;
  to: SelectionEndpointBounds;
};

export type ResolvedTouchPoint = {
  pageIdx: number;
  x: number;
  y: number;
};

export type TouchMenuPosition = {
  x: number;
  y: number;
  placement: Placement;
};

const TOUCH_MENU_VIEWPORT_PADDING = 8;
const TOUCH_MENU_ESTIMATED_HEIGHT = 56;
const VIEWPORT_COORDINATE_EPSILON = 0.5;

const distance = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);
const isWithin = (value: number, min: number, max: number) =>
  value >= min - VIEWPORT_COORDINATE_EPSILON && value <= max + VIEWPORT_COORDINATE_EPSILON;

const resolveViewportCoordinate = (value: number, offset: number, size: number) => {
  const min = offset;
  const max = offset + size;
  const layoutCandidate = value;
  const visualCandidate = value + offset;
  const layoutInRange = isWithin(layoutCandidate, min, max);
  const visualInRange = isWithin(visualCandidate, min, max);

  if (layoutInRange && !visualInRange) {
    return layoutCandidate;
  }

  if (visualInRange && !layoutInRange) {
    return visualCandidate;
  }

  const viewportCenter = (min + max) / 2;
  return Math.abs(layoutCandidate - viewportCenter) <= Math.abs(visualCandidate - viewportCenter) ? layoutCandidate : visualCandidate;
};

const isTouchLikePointer = (e: PointerEvent) => {
  return e.pointerType === 'touch';
};

export const getOrderedSelectionHandles = (selection: Selection | null): OrderedSelectionHandles | null => {
  if (!selection || selection.collapsed || !selection.anchorBounds || !selection.headBounds) {
    return null;
  }

  if (selection.cmp >= 0) {
    return {
      from: selection.anchorBounds,
      to: selection.headBounds,
    };
  }

  return {
    from: selection.headBounds,
    to: selection.anchorBounds,
  };
};

const computeTouchContextMenuPosition = (selection: Selection | null, pageContainerEls: HTMLDivElement[]): TouchMenuPosition | null => {
  const handles = getOrderedSelectionHandles(selection);
  if (!handles) {
    return null;
  }

  const fromPage = pageContainerEls[handles.from.pageIdx];
  const toPage = pageContainerEls[handles.to.pageIdx];
  if (!fromPage || !toPage) {
    return null;
  }

  if (typeof window === 'undefined') {
    return null;
  }

  const fromRect = fromPage.getBoundingClientRect();
  const toRect = toPage.getBoundingClientRect();

  const fromLeft = fromRect.left + handles.from.bounds.x;
  const fromRight = fromLeft + handles.from.bounds.width;
  const fromTop = fromRect.top + handles.from.bounds.y;
  const fromBottom = fromTop + handles.from.bounds.height;

  const toLeft = toRect.left + handles.to.bounds.x;
  const toRight = toLeft + handles.to.bounds.width;
  const toTop = toRect.top + handles.to.bounds.y;
  const toBottom = toTop + handles.to.bounds.height;

  const selectionLeft = Math.min(fromLeft, toLeft);
  const selectionRight = Math.max(fromRight, toRight);
  const selectionTop = Math.min(fromTop, toTop);
  const selectionBottom = Math.max(fromBottom, toBottom);

  const visualViewport = window.visualViewport;
  const viewportLeft = visualViewport?.offsetLeft ?? 0;
  const viewportTop = visualViewport?.offsetTop ?? 0;
  const viewportWidth = visualViewport?.width ?? window.innerWidth;
  const viewportHeight = visualViewport?.height ?? window.innerHeight;
  const viewportRight = viewportLeft + viewportWidth;
  const viewportBottom = viewportTop + viewportHeight;

  const centerX = resolveViewportCoordinate((selectionLeft + selectionRight) / 2, viewportLeft, viewportWidth);
  const resolvedTop = resolveViewportCoordinate(selectionTop, viewportTop, viewportHeight);
  const resolvedBottom = resolveViewportCoordinate(selectionBottom, viewportTop, viewportHeight);
  const spaceAbove = resolvedTop - viewportTop;
  const spaceBelow = viewportBottom - resolvedBottom;

  const placement: Placement = spaceAbove >= TOUCH_MENU_ESTIMATED_HEIGHT || spaceAbove >= spaceBelow ? 'top' : 'bottom';
  const anchorY = placement === 'top' ? resolvedTop : resolvedBottom;

  return {
    x: clamp(centerX, viewportLeft + TOUCH_MENU_VIEWPORT_PADDING, viewportRight - TOUCH_MENU_VIEWPORT_PADDING),
    y: clamp(anchorY, viewportTop + TOUCH_MENU_VIEWPORT_PADDING, viewportBottom - TOUCH_MENU_VIEWPORT_PADDING),
    placement,
  };
};

type TouchPhase = 'idle' | 'pressing' | 'doubleTapPending' | 'doubleTapDragging' | 'handleDragging' | 'dndArmed';

type TapRecord = {
  time: number;
  x: number;
  y: number;
};

type PressRecord = {
  time: number;
  x: number;
  y: number;
  selectionHit: boolean;
};

type HapticKind = 'tap' | 'selection' | 'handle' | 'dragArm' | 'dragStart';

export class TouchGestureController {
  #editor: Editor;
  #phase: TouchPhase = $state('idle');
  #activePointerId: number | null = $state(null);
  #lastTap: TapRecord | null = null;
  #press: PressRecord | null = null;
  #dragAnchor: SelectionEndpointBounds | null = null;
  #doubleTapInitialRange: { anchor: Position; head: Position } | null = null;
  #doubleTapStart: { x: number; y: number } | null = null;
  #lastClientPoint: { x: number; y: number } | null = null;
  #longPressTimer: ReturnType<typeof setTimeout> | null = null;
  #autoScrollCleanup: (() => void) | null = null;
  #suppressTapOnPointerUp = false;
  #movedPastTapThreshold = false;
  #readOnlyDragStarted = false;
  #dragArmed = false;
  #dragCandidate = false;
  #contextMenuRequestId = 0;
  #suppressNativeContextMenuUntil = 0;
  #wasTouchMenuOpenOnPointerDown = false;
  #lastHapticAt = 0;

  constructor(editor: Editor) {
    this.#editor = editor;
  }

  get gestureActive(): boolean {
    return this.#editor.readOnly && (this.#activePointerId !== null || this.#phase !== 'idle');
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

  get isDoubleTapSelectionDragActive(): boolean {
    return this.#phase === 'doubleTapPending' || this.#phase === 'doubleTapDragging';
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

  #nextContextMenuRequestId(): number {
    return ++this.#contextMenuRequestId;
  }

  #isContextMenuRequestActive(requestId: number): boolean {
    return this.#contextMenuRequestId === requestId;
  }

  cancelContextMenuRequest(): void {
    this.#contextMenuRequestId++;
  }

  resetReadOnlyTouchState(): void {
    this.cancelSession();
    this.#setDragArmed(false);
    this.#setDragCandidate(false);
    this.cancelContextMenuRequest();
  }

  #setPhase(next: TouchPhase): void {
    this.#phase = next;
  }

  handlePointerDown(e: PointerEvent, resolved: ResolvedTouchPoint | null): void {
    if (!isTouchLikePointer(e) || !e.isPrimary) {
      return;
    }

    this.#clearNativeSelection();

    if (this.#activePointerId !== null && this.#activePointerId !== e.pointerId) {
      this.cancelSession();
      return;
    }

    this.#activePointerId = e.pointerId;
    const selectionHit = resolved ? this.#editor.isSelectionHit(resolved.pageIdx, resolved.x, resolved.y) : false;
    this.#press = {
      time: performance.now(),
      x: e.clientX,
      y: e.clientY,
      selectionHit,
    };
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#suppressTapOnPointerUp = false;
    this.#movedPastTapThreshold = false;
    this.#readOnlyDragStarted = false;
    this.#dragAnchor = null;
    this.#doubleTapInitialRange = null;
    this.#doubleTapStart = null;
    this.#setDragArmed(false);
    this.#setDragCandidate(false);
    this.#stopAutoScroll();
    this.#clearLongPressTimer();
    this.#suppressNativeContextMenuUntil = performance.now() + TOUCH_LONG_PRESS_MS + 600;
    this.#wasTouchMenuOpenOnPointerDown = this.#editor.contextMenu.isOpen && this.#editor.contextMenu.source === 'touch';
    if (this.#wasTouchMenuOpenOnPointerDown) {
      this.#editor.closeContextMenu();
    }

    const isDoubleTap =
      this.#lastTap &&
      this.#press.time - this.#lastTap.time <= TOUCH_DOUBLE_TAP_INTERVAL_MS &&
      distance(this.#lastTap, this.#press) <= TOUCH_DOUBLE_TAP_DISTANCE_PX;

    if (isDoubleTap && resolved) {
      this.#dispatchPointerClick(resolved, 2);
      this.#setPhase('doubleTapPending');
      this.#doubleTapStart = { x: e.clientX, y: e.clientY };
      this.#suppressTapOnPointerUp = true;
      this.#lastTap = null;
      this.#vibrate('tap');
      return;
    }

    this.#setPhase('pressing');
    this.#startLongPressTimer();
  }

  handlePointerMove(e: PointerEvent): void {
    if (this.#activePointerId !== e.pointerId) {
      return;
    }

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };

    if (this.#phase === 'pressing') {
      if (this.#press && distance(this.#press, this.#lastClientPoint) > TOUCH_LONG_PRESS_CANCEL_DISTANCE_PX) {
        this.#movedPastTapThreshold = true;
        this.#clearLongPressTimer();
        this.#setDragCandidate(false);
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

      this.#setPhase('doubleTapDragging');
    }

    if (this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging') {
      this.#dispatchDragSelectionAtCurrentPoint();
      this.#updateAutoScroll();
      e.preventDefault();
      return;
    }

    if (this.#phase === 'dndArmed') {
      this.#updateAutoScroll();
    }
  }

  handlePointerUp(e: PointerEvent, resolved: ResolvedTouchPoint | null): void {
    if (this.#activePointerId !== e.pointerId) {
      return;
    }

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#clearLongPressTimer();
    this.#stopAutoScroll();

    const fallbackResolved = this.#editor.resolvePointerCoordinateFromClient(e.clientX, e.clientY);
    const finalResolved = resolved ?? fallbackResolved;
    const now = performance.now();

    switch (this.#phase) {
      case 'doubleTapDragging':
      case 'handleDragging': {
        this.#dispatchDragSelectionAtCurrentPoint();
        this.#openTouchContextMenuFromSelectionDeferred();
        this.#lastTap = null;
        break;
      }
      case 'doubleTapPending': {
        this.#openTouchContextMenuFromSelectionDeferred();
        this.#lastTap = null;
        break;
      }
      case 'dndArmed': {
        if (!this.#readOnlyDragStarted) {
          this.#openTouchContextMenuFromSelection();
        }
        this.#lastTap = null;
        break;
      }
      case 'pressing': {
        if (!this.#suppressTapOnPointerUp && !this.#movedPastTapThreshold && this.#press) {
          if (finalResolved) {
            const selectionHit = this.#editor.isSelectionHit(finalResolved.pageIdx, finalResolved.x, finalResolved.y);
            if (selectionHit) {
              if (!this.#wasTouchMenuOpenOnPointerDown) {
                this.#openTouchContextMenuFromSelection();
              }
            } else {
              this.#dispatchPointerClick(finalResolved, 1);
              this.#editor.closeContextMenu();
            }
            this.#lastTap = {
              time: now,
              x: e.clientX,
              y: e.clientY,
            };
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
    if (this.#activePointerId !== null && this.#activePointerId !== e.pointerId) {
      return;
    }

    this.cancelSession();
  }

  handleSelectionHandlePointerDown(type: SelectionHandleKind, e: PointerEvent): void {
    if (!isTouchLikePointer(e)) {
      return;
    }

    const handles = getOrderedSelectionHandles(this.#editor.selection);
    if (!handles) {
      return;
    }

    this.#activePointerId = e.pointerId;
    this.#setPhase('handleDragging');
    this.#dragAnchor = type === 'from' ? handles.to : handles.from;
    this.#doubleTapInitialRange = null;
    this.#suppressTapOnPointerUp = true;
    this.#movedPastTapThreshold = true;
    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#clearLongPressTimer();
    this.#stopAutoScroll();
    this.#editor.closeContextMenu();
    this.#setDragCandidate(false);
    this.#setDragArmed(false);
    this.#suppressNativeContextMenuUntil = performance.now() + 600;
    this.#vibrate('handle');
    this.#dispatchDragSelectionAtCurrentPoint();
  }

  handleSelectionHandlePointerMove(e: PointerEvent): void {
    if (this.#phase !== 'handleDragging' || this.#activePointerId !== e.pointerId) {
      return;
    }

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#dispatchDragSelectionAtCurrentPoint();
    this.#updateAutoScroll();
    e.preventDefault();
  }

  handleSelectionHandlePointerUp(e: PointerEvent): void {
    if (this.#phase !== 'handleDragging' || this.#activePointerId !== e.pointerId) {
      return;
    }

    this.#lastClientPoint = { x: e.clientX, y: e.clientY };
    this.#dispatchDragSelectionAtCurrentPoint();
    this.#openTouchContextMenuFromSelectionDeferred();
    this.#resetSession();
  }

  handleNativeDragStart(): void {
    if (this.#phase !== 'dndArmed') {
      return;
    }

    this.#readOnlyDragStarted = true;
    this.#editor.closeContextMenu();
    this.#clearLongPressTimer();
    this.#stopAutoScroll();
    this.#vibrate('dragStart');
  }

  handleNativeDragEnd(): void {
    this.#readOnlyDragStarted = false;
    this.#resetSession();
  }

  cancelSession(): void {
    this.#lastTap = null;
    this.#resetSession();
  }

  destroy(): void {
    this.#resetSession();
  }

  #startLongPressTimer(): void {
    this.#clearLongPressTimer();
    this.#longPressTimer = setTimeout(() => {
      if (this.#phase !== 'pressing' || !this.#press) {
        return;
      }

      const point = this.#lastClientPoint ?? this.#press;
      const resolved = this.#editor.resolvePointerCoordinateFromClient(point.x, point.y);
      if (!resolved) {
        return;
      }

      if (this.#press.selectionHit) {
        this.#setPhase('dndArmed');
        this.#setDragCandidate(true);
        this.#setDragArmed(true);
        this.#suppressTapOnPointerUp = true;
        this.#vibrate('dragArm');
        this.#updateAutoScroll();
        return;
      }

      this.#dispatchPointerClick(resolved, 2);
      this.#setPhase('doubleTapPending');
      this.#doubleTapStart = { x: point.x, y: point.y };
      this.#suppressTapOnPointerUp = true;
      this.#vibrate('selection');
    }, TOUCH_LONG_PRESS_MS);
  }

  #clearLongPressTimer(): void {
    if (this.#longPressTimer !== null) {
      clearTimeout(this.#longPressTimer);
      this.#longPressTimer = null;
    }
  }

  #openTouchContextMenuFromSelection(): void {
    const position = computeTouchContextMenuPosition(this.#editor.selection, this.#editor.pageContainerEls);
    if (!position) {
      this.#editor.closeContextMenu();
      return;
    }

    this.#editor.openContextMenu({ x: position.x, y: position.y, source: 'touch', placement: position.placement });
  }

  #openTouchContextMenuFromSelectionDeferred(): void {
    const requestId = this.#nextContextMenuRequestId();
    this.#editor.runAfterSettled(() => {
      if (!this.#isContextMenuRequestActive(requestId)) {
        return;
      }

      this.#openTouchContextMenuFromSelection();
    });
  }

  #dispatchPointerClick(point: ResolvedTouchPoint, clickCount: number): void {
    this.#editor.dispatch({
      type: 'pointerDown',
      pageIdx: point.pageIdx,
      x: point.x,
      y: point.y,
      clickCount,
      button: 'primary',
      modifier: { shift: false, ctrl: false, alt: false, meta: false },
    });
    this.#editor.dispatch({
      type: 'pointerUp',
      pageIdx: point.pageIdx,
      x: point.x,
      y: point.y,
      button: 'primary',
      modifier: { shift: false, ctrl: false, alt: false, meta: false },
    });
  }

  #resolveDoubleTapDragSelectionContext(): {
    anchor: SelectionEndpointBounds;
    doubleTapInitialRange: { anchor: Position; head: Position };
  } | null {
    if (!this.#doubleTapInitialRange) {
      const selection = this.#editor.selection;
      if (!selection || selection.collapsed) {
        return null;
      }

      this.#doubleTapInitialRange = { anchor: selection.anchor, head: selection.head };
    }

    if (!this.#dragAnchor) {
      const handles = getOrderedSelectionHandles(this.#editor.selection);
      if (!handles) {
        return null;
      }
      this.#dragAnchor = handles.from;
    }

    if (!this.#dragAnchor || !this.#doubleTapInitialRange) {
      return null;
    }

    return {
      anchor: this.#dragAnchor,
      doubleTapInitialRange: this.#doubleTapInitialRange,
    };
  }

  #dispatchDragSelectionAtCurrentPoint(): void {
    const current = this.#lastClientPoint;
    if (!current) {
      return;
    }

    const resolved = this.#editor.resolvePointerCoordinateFromClient(current.x, current.y);
    if (!resolved) {
      return;
    }

    let anchor = this.#dragAnchor;
    let doubleTapInitialRange: { anchor: Position; head: Position } | undefined;

    if (this.#phase === 'doubleTapDragging') {
      const context = this.#resolveDoubleTapDragSelectionContext();
      if (!context) {
        return;
      }

      anchor = context.anchor;
      doubleTapInitialRange = context.doubleTapInitialRange;
    } else if (!anchor) {
      return;
    }

    this.#editor.dispatch({
      type: 'extendSelectionTo',
      anchorPageIdx: anchor.pageIdx,
      anchorX: anchor.bounds.x,
      anchorY: anchor.bounds.y + anchor.bounds.height / 2,
      headPageIdx: resolved.pageIdx,
      headX: resolved.x,
      headY: resolved.y,
      doubleTapInitialRange,
    });
  }

  #updateAutoScroll(): void {
    if (!this.#shouldAutoScroll() || this.#lastClientPoint === null || this.#editor.scrollViewport === null) {
      this.#stopAutoScroll();
      return;
    }

    if (this.#autoScrollCleanup !== null) {
      this.#autoScrollCleanup();
      this.#autoScrollCleanup = null;
    }

    const pointer = this.#lastClientPoint;
    this.#autoScrollCleanup =
      handleDragScroll(this.#editor.scrollViewport, true, {
        scrollZoneSize: TOUCH_EDGE_SCROLL_THRESHOLD_PX,
        minScrollSpeed: TOUCH_EDGE_MIN_SCROLL_SPEED,
        maxScrollSpeed: TOUCH_EDGE_MAX_SCROLL_SPEED,
        axis: 'both',
        initialPointer: { clientX: pointer.x, clientY: pointer.y },
        onScrollThrottleMs: 16,
        onScroll: (clientX, clientY) => {
          this.#lastClientPoint = { x: clientX, y: clientY };
          if (this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging') {
            this.#dispatchDragSelectionAtCurrentPoint();
          }
        },
      }) ?? null;
  }

  #stopAutoScroll(): void {
    if (this.#autoScrollCleanup !== null) {
      this.#autoScrollCleanup();
      this.#autoScrollCleanup = null;
    }
  }

  #shouldAutoScroll(): boolean {
    return this.#phase === 'doubleTapDragging' || this.#phase === 'handleDragging' || this.#phase === 'dndArmed';
  }

  #resetSession(): void {
    this.#setPhase('idle');
    this.#activePointerId = null;
    this.#press = null;
    this.#dragAnchor = null;
    this.#doubleTapInitialRange = null;
    this.#doubleTapStart = null;
    this.#lastClientPoint = null;
    this.#suppressTapOnPointerUp = false;
    this.#movedPastTapThreshold = false;
    this.#readOnlyDragStarted = false;
    this.#wasTouchMenuOpenOnPointerDown = false;
    this.#setDragCandidate(false);
    this.#setDragArmed(false);
    this.#clearLongPressTimer();
    this.#stopAutoScroll();
  }

  #setDragArmed(armed: boolean): void {
    if (this.#dragArmed === armed) {
      return;
    }

    this.#dragArmed = armed;
  }

  #setDragCandidate(candidate: boolean): void {
    if (this.#dragCandidate === candidate) {
      return;
    }

    this.#dragCandidate = candidate;
  }

  #vibrate(kind: HapticKind): void {
    if (typeof navigator === 'undefined' || typeof navigator.vibrate !== 'function') {
      return;
    }

    if (typeof document !== 'undefined' && document.visibilityState !== 'visible') {
      return;
    }

    if (typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      return;
    }

    const now = performance.now();
    const minInterval = kind === 'tap' ? 40 : 90;
    if (now - this.#lastHapticAt < minInterval) {
      return;
    }
    this.#lastHapticAt = now;

    const pattern: number | number[] =
      kind === 'dragArm' ? [10, 20, 12] : kind === 'selection' ? 14 : kind === 'handle' ? 12 : kind === 'dragStart' ? 10 : 8;

    navigator.vibrate(pattern);
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
}
