import { match } from 'ts-pattern';
import { EditorEdgeAutoScroll } from '../edge-auto-scroll';
import { isSelectionCollapsed } from '../geometry';
import type { InputModifiers, InteractiveHit, Position, Rect, Selection, SelectionPointUnit } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

const DRAG_START_THRESHOLD_PX = 5;

const pointInRect = (x: number, y: number, r: Rect): boolean => x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height;

type LocalPoint = { page: number; x: number; y: number };
type DragPoint = LocalPoint & { clientX: number; clientY: number };

const interactiveTarget = (editor: Editor, hit: InteractiveHit | undefined, local: { x: number; y: number }) => {
  if (!hit) return;
  return match(hit)
    .with({ type: 'fold_title' }, (hit) => {
      const selectableTitle = !editor.readOnly || (editor.nativeSelection && !editor.protectContent);
      if (selectableTitle && hit.text_rect && pointInRect(local.x, local.y, hit.text_rect)) return;
      return hit;
    })
    .with({ type: 'callout_icon' }, (hit) => (editor.readOnly ? undefined : hit))
    .exhaustive();
};

export const handlePointerDown: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  // A new press, including a second finger, invalidates the previous click target.
  PointerState.of(editor).clickTarget = undefined;
  if (!e.isPrimary) return;

  if (e.button !== 0) return;

  const local = editor.clientToLocal(e.clientX, e.clientY);
  if (!local) {
    return;
  }

  const hit = interactiveTarget(editor, editor.interactiveHitTest(local.page, local.x, local.y), local);
  if (hit) {
    // The browser decides whether this gesture produces a click. Retain only
    // its engine target because multiple canvas controls share one DOM node.
    PointerState.of(editor).clickTarget = { pointerId: e.pointerId, hit };
    // Keep mouse/pen presses on controls from starting native text selection.
    if (editor.nativeSelection && e.pointerType !== 'touch') e.preventDefault();
    return;
  }
  // The viewer shares interactive hit testing, then leaves text selection to the browser.
  if (editor.nativeSelection) return;

  const { page, x, y } = local;
  const count = PointerState.of(editor).resolveClickCount(e);
  const modifiers: InputModifiers = { shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey };
  const appliedSelection = editor.appliedSnapshot.selection;

  const nativeDragCandidate =
    count === 1 && !modifiers.shift && !isSelectionCollapsed(appliedSelection) && editor.selectionHitTest(page, x, y);
  if (nativeDragCandidate) {
    const target = e.currentTarget;
    editor.beginNativeDragAdmission();
    target.removeAttribute('tabindex');
    setTimeout(() => {
      target.setAttribute('tabindex', '0');
    }, 0);
  } else {
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  const state = PointerState.of(editor);
  let interactionSelection = appliedSelection;
  if (!nativeDragCandidate) {
    const update = editor.updateNow(() => {
      if (count === 1 && appliedSelection && modifiers.shift) {
        editor.enqueue({
          type: 'selection',
          op: {
            type: 'extend_to',
            anchor: appliedSelection.anchor,
            head_page: page,
            head_x: x,
            head_y: y,
            base_selection: undefined,
            allow_collapse: true,
          },
        });
      } else if (count === 1) {
        editor.enqueue({ type: 'selection', op: { type: 'set_at', page, x, y } });
      } else {
        editor.enqueue({
          type: 'selection',
          op: {
            type: 'select_unit_at',
            page,
            x,
            y,
            unit: count === 2 ? 'word' : 'paragraph',
          },
        });
      }
    });
    interactionSelection = update?.snapshot.selection;
    if (!editor.readOnly) {
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'pointer_cursor_guard' });
    }
  }
  state.markPointerDown(
    e.pointerId,
    !nativeDragCandidate,
    { page, x, y },
    count,
    modifiers,
    nativeDragCandidate,
    interactionSelection,
    e.pointerType === 'mouse' && count > 1 ? (count === 2 ? 'word' : 'paragraph') : undefined,
  );
  if (!nativeDragCandidate) {
    editor.suspendToolbarSync();
  }
};

export const handlePointerMove: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (editor.nativeSelection) return;

  editor.updatePointerHover(e.clientX, e.clientY);
  if (!e.currentTarget.hasPointerCapture(e.pointerId)) {
    return;
  }

  const local = editor.clientToLocal(e.clientX, e.clientY);
  if (!local) {
    return;
  }

  e.preventDefault();
  PointerState.of(editor).enqueueMoveThrottled(editor, { ...local, clientX: e.clientX, clientY: e.clientY });
};

export const handlePointerUp: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (editor.nativeSelection) return;

  const state = PointerState.of(editor);
  if (!state.hasActivePointer(e.pointerId)) {
    return;
  }

  state.finishPointerUp(editor, e.pointerId, { clientX: e.clientX, clientY: e.clientY });
  state.releasePointer(e.currentTarget, e.pointerId);
  editor.resumeToolbarSync();
  editor.endNativeDragAdmission({ restoreFocus: true });
};

export const handleClick: EditorEventHandler<HTMLElement, MouseEvent> = (editor, e) => {
  const state = PointerState.of(editor);
  const target = state.clickTarget;
  if (target && 'pointerId' in e && e.pointerId !== target.pointerId) return;
  state.clickTarget = undefined;
  if (e.button !== 0) return;

  const local = editor.clientToLocal(e.clientX, e.clientY);
  if (!local) return;
  if (target) {
    const hit = interactiveTarget(editor, editor.interactiveHitTest(local.page, local.x, local.y), local);
    if (!hit || hit.type !== target.hit.type || hit.id !== target.hit.id) return;
    match(hit)
      .with({ type: 'fold_title' }, ({ id }) => editor.enqueue({ type: 'view', op: { type: 'toggle_fold', id } }))
      .with({ type: 'callout_icon' }, ({ id, next_variant }) =>
        editor.enqueue({ type: 'node', op: { type: 'set_attrs', id, attrs: { type: 'callout', variant: next_variant } } }),
      )
      .exhaustive();
    return;
  }
  if (editor.nativeSelection || !editor.commentClickHandler) return;
  const id = editor.commentIdAt(local.page, local.x, local.y);
  if (id !== null) editor.commentClickHandler(id);
};

export const handlePointerCancel: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  const state = PointerState.of(editor);
  if (state.clickTarget?.pointerId === e.pointerId) state.clickTarget = undefined;
  if (!state.hasActivePointer(e.pointerId)) return;

  state.cancelPointer(e.pointerId);
  state.releasePointer(e.currentTarget, e.pointerId);
  editor.resumeToolbarSync();
  editor.endNativeDragAdmission({ restoreFocus: false });
};

export const handlePointerCaptureLost: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  const state = PointerState.of(editor);
  if (!state.hasActivePointer(e.pointerId)) return;

  state.cancelPointer(e.pointerId);
  editor.resumeToolbarSync();
  editor.endNativeDragAdmission({ restoreFocus: false });
};

export const cancelPointerInteraction = (editor: Editor): void => {
  const state = PointerState.of(editor);
  if (!state.cancelActivePointer()) return;

  editor.resumeToolbarSync();
  editor.endNativeDragAdmission({ restoreFocus: false });
};

export const markNativeSelectionDragStarted = (editor: Editor): void => {
  PointerState.of(editor).markNativeDragStarted();
};

class PointerState {
  static #instances = new WeakMap<Editor, PointerState>();

  static of(editor: Editor): PointerState {
    let state = this.#instances.get(editor);
    if (!state) {
      state = new PointerState();
      this.#instances.set(editor, state);
    }
    return state;
  }

  #clickTime = 0;
  #clickX = 0;
  #clickY = 0;
  #clickCount = 0;

  #dragPending: DragPoint | null = null;
  #dragScheduled = false;
  #edgeAutoScroll = new EditorEdgeAutoScroll();
  #session: {
    pointerId: number;
    captured: boolean;
    down: LocalPoint;
    anchor: Position | null;
    baseSelection: Selection | undefined;
    unit: SelectionPointUnit | undefined;
    nativeDragCandidate: boolean;
    nativeDragStarted: boolean;
    dragging: boolean;
  } | null = null;

  clickTarget: { pointerId: number; hit: InteractiveHit } | undefined;

  #flushDragPending(editor: Editor): void {
    const point = this.#dragPending;
    this.#dragPending = null;
    if (!point) return;

    if (editor.updateNow(() => this.#extendSelectionTo(editor, point, { respectThreshold: true })) !== null) {
      this.#edgeAutoScroll.update(editor, point, (clientX, clientY) => {
        if (editor.destroyed) {
          this.#edgeAutoScroll.stop();
          return;
        }

        const local = editor.clientToLocal(clientX, clientY);
        if (!local) return;

        editor.updateNow(() => this.#extendSelectionTo(editor, { ...local, clientX, clientY }, { respectThreshold: false }));
      });
    }
  }

  #extendSelectionTo(editor: Editor, point: DragPoint, { respectThreshold }: { respectThreshold: boolean }): boolean {
    if (!this.#session?.anchor) return false;
    const { down } = this.#session;
    const dx = point.x - down.x;
    const dy = point.y - down.y;
    if (
      respectThreshold &&
      !this.#session.dragging &&
      point.page === down.page &&
      dx * dx + dy * dy < DRAG_START_THRESHOLD_PX * DRAG_START_THRESHOLD_PX
    ) {
      return false;
    }

    this.#session.dragging = true;
    this.#clickCount = 0;
    editor.enqueue({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: this.#session.anchor,
        head_page: point.page,
        head_x: point.x,
        head_y: point.y,
        base_selection: this.#session.baseSelection,
        unit: this.#session.unit,
        allow_collapse: this.#session.baseSelection === undefined,
      },
    });
    return true;
  }

  resolveClickCount(e: PointerEvent): number {
    const now = e.timeStamp;
    const dx = e.clientX - this.#clickX;
    const dy = e.clientY - this.#clickY;

    if (now - this.#clickTime < 500 && dx * dx + dy * dy < 25) {
      this.#clickCount++;
    } else {
      this.#clickCount = 1;
    }

    this.#clickTime = now;
    this.#clickX = e.clientX;
    this.#clickY = e.clientY;

    return this.#clickCount;
  }

  enqueueMoveThrottled(editor: Editor, point: DragPoint) {
    this.#dragPending = point;

    if (!this.#dragScheduled) {
      this.#dragScheduled = true;
      requestAnimationFrame(() => {
        this.#dragScheduled = false;
        this.#flushDragPending(editor);
      });
    }
  }

  markPointerDown(
    pointerId: number,
    captured: boolean,
    down: LocalPoint,
    count: number,
    modifiers: InputModifiers,
    nativeDragCandidate: boolean,
    selection: Selection | undefined,
    unit: SelectionPointUnit | undefined,
  ) {
    const selectionCollapsed = isSelectionCollapsed(selection);
    const canExtend = !nativeDragCandidate && (count > 1 ? selection !== undefined : modifiers.shift || selectionCollapsed);
    this.#session = {
      pointerId,
      captured,
      down,
      anchor: canExtend ? (selection?.anchor ?? null) : null,
      baseSelection: selection && !selectionCollapsed && count > 1 ? selection : undefined,
      unit,
      nativeDragCandidate,
      nativeDragStarted: false,
      dragging: false,
    };
  }

  hasActivePointer(pointerId: number): boolean {
    return this.#session?.pointerId === pointerId;
  }

  releasePointer(target: HTMLElement, pointerId: number): void {
    if (target.hasPointerCapture(pointerId)) {
      target.releasePointerCapture(pointerId);
    }
  }

  finishPointerUp(editor: Editor, pointerId: number, pointer: { clientX: number; clientY: number }): void {
    const session = this.#session;
    if (!session || session.pointerId !== pointerId) return;

    this.#flushDragPending(editor);
    if (session.dragging) {
      const local = editor.clientToLocal(pointer.clientX, pointer.clientY);
      if (local) {
        editor.updateNow(() => this.#extendSelectionTo(editor, { ...local, ...pointer }, { respectThreshold: false }));
      }
    }
    if (session.nativeDragCandidate && !session.nativeDragStarted) {
      editor.updateNow(() =>
        editor.enqueue({
          type: 'selection',
          op: { type: 'set_at', page: session.down.page, x: session.down.x, y: session.down.y },
        }),
      );
      if (!editor.readOnly) {
        editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'pointer_cursor_guard' });
      }
    }
    this.#edgeAutoScroll.stop();
    this.#session = null;
  }

  cancelPointer(pointerId: number): void {
    if (this.#session?.pointerId !== pointerId) return;
    this.#dragPending = null;
    this.#edgeAutoScroll.stop();
    this.#session = null;
  }

  cancelActivePointer(): boolean {
    this.clickTarget = undefined;
    const pointerId = this.#session?.pointerId;
    if (pointerId === undefined) return false;
    this.cancelPointer(pointerId);
    return true;
  }

  markNativeDragStarted(): void {
    if (this.#session?.nativeDragCandidate) {
      this.#session.nativeDragStarted = true;
    }
  }
}
