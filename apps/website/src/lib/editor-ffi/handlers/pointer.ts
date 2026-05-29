import type { InputModifiers, InteractiveHit, Position, Rect, Selection } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

const DRAG_START_THRESHOLD_PX = 5;

const pointInRect = (x: number, y: number, r: Rect): boolean => x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height;

type LocalPoint = { page: number; x: number; y: number };

export const tryHandleInteractiveHit = (editor: Editor, hit: InteractiveHit, local: { x: number; y: number }): boolean => {
  const editMode = !editor.readOnly;
  if (hit.type === 'fold_title') {
    const onText = editMode && hit.text_rect !== undefined && pointInRect(local.x, local.y, hit.text_rect);
    if (!onText) {
      editor.enqueue({ type: 'view', op: { type: 'toggle_fold', id: hit.id } });
      return true;
    }
  } else if (hit.type === 'callout_icon' && editMode) {
    editor.enqueue({ type: 'node', op: { type: 'set_attrs', id: hit.id, attrs: { type: 'callout', variant: hit.next_variant } } });
    return true;
  }
  return false;
};

export const handlePointerDown: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (e.pointerType === 'touch') {
    const local = editor.clientToLocal(e.clientX, e.clientY);
    const resolved = local ? { page: local.page, x: local.x, y: local.y } : null;
    e.currentTarget.setPointerCapture(e.pointerId);
    editor.gesture.handlePointerDown(e, resolved);
    return;
  }

  if (e.button !== 0) return;

  const local = editor.clientToLocal(e.clientX, e.clientY);
  if (!local) {
    return;
  }

  const hit = editor.interactiveHitTest(local.page, local.x, local.y);
  if (hit && tryHandleInteractiveHit(editor, hit, { x: local.x, y: local.y })) {
    return;
  }

  const { page, x, y } = local;
  const count = PointerState.of(editor).resolveClickCount(e);
  const modifiers: InputModifiers = { shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey };

  const selectionHit = !editor.isSelectionCollapsed && editor.selectionHitTest(page, x, y);
  const nativeDragCandidate = !editor.isSelectionCollapsed && selectionHit;
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
  if (!nativeDragCandidate) {
    if (count === 1 && modifiers.shift && editor.selection) {
      editor.enqueue({
        type: 'selection',
        op: {
          type: 'extend_to',
          anchor: editor.selection.anchor,
          head_page: page,
          head_x: x,
          head_y: y,
          base_selection: undefined,
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
    editor.flush();
  }
  state.markPointerDown(editor, e.pointerId, !nativeDragCandidate, { page, x, y }, count, modifiers, nativeDragCandidate);
};

export const handlePointerMove: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (e.pointerType === 'touch') {
    editor.gesture.handlePointerMove(e);
    return;
  }

  editor.updatePointerHover(e.clientX, e.clientY);

  if (!e.currentTarget.hasPointerCapture(e.pointerId)) {
    return;
  }

  const local = editor.clientToLocal(e.clientX, e.clientY);
  if (!local) {
    return;
  }

  e.preventDefault();
  PointerState.of(editor).enqueueMoveThrottled(editor, local);
};

export const handlePointerUp: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (e.pointerType === 'touch') {
    editor.gesture.handlePointerUp(e);
    return;
  }

  const state = PointerState.of(editor);
  if (!state.hasActivePointer(e.pointerId)) {
    return;
  }

  state.releasePointer(e.currentTarget, e.pointerId);
  state.finishPointerUp(editor, e.pointerId);
  editor.flush();
  editor.endNativeDragAdmission({ restoreFocus: true });
};

export const handlePointerCancel: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (e.pointerType === 'touch') {
    editor.gesture.handlePointerCancel(e);
    return;
  }

  const state = PointerState.of(editor);
  if (!state.hasActivePointer(e.pointerId)) {
    return;
  }

  state.releasePointer(e.currentTarget, e.pointerId);
  state.cancelPointer(e.pointerId);
  editor.flush();
  editor.endNativeDragAdmission({ restoreFocus: false });
};

export const markNativeSelectionDragStarted = (editor: Editor): void => {
  PointerState.of(editor).markNativeDragStarted();
};

class PointerState {
  static #instances = new WeakMap<Editor, PointerState>();

  #clickTime = 0;
  #clickX = 0;
  #clickY = 0;
  #clickCount = 0;

  #dragPending: LocalPoint | null = null;
  #dragScheduled = false;
  #session: {
    pointerId: number;
    captured: boolean;
    down: LocalPoint;
    anchor: Position | null;
    baseSelection: Selection | undefined;
    nativeDragCandidate: boolean;
    nativeDragStarted: boolean;
    dragging: boolean;
  } | null = null;

  static of(editor: Editor): PointerState {
    let state = this.#instances.get(editor);
    if (!state) {
      state = new PointerState();
      this.#instances.set(editor, state);
    }
    return state;
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

  enqueueMoveThrottled(editor: Editor, point: LocalPoint) {
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
    editor: Editor,
    pointerId: number,
    captured: boolean,
    down: LocalPoint,
    count: number,
    modifiers: InputModifiers,
    nativeDragCandidate: boolean,
  ) {
    const selection = editor.selection;
    const canExtend = !nativeDragCandidate && (count > 1 ? selection !== undefined : modifiers.shift || editor.isSelectionCollapsed);
    this.#session = {
      pointerId,
      captured,
      down,
      anchor: canExtend ? (selection?.anchor ?? null) : null,
      baseSelection: count > 1 && selection && !editor.isSelectionCollapsed ? selection : undefined,
      nativeDragCandidate,
      nativeDragStarted: false,
      dragging: false,
    };
  }

  hasActivePointer(pointerId: number): boolean {
    return this.#session?.pointerId === pointerId;
  }

  releasePointer(target: HTMLElement, pointerId: number): void {
    if (this.#session?.captured && this.#session.pointerId === pointerId && target.hasPointerCapture(pointerId)) {
      target.releasePointerCapture(pointerId);
    }
  }

  finishPointerUp(editor: Editor, pointerId: number): void {
    const session = this.#session;
    if (!session || session.pointerId !== pointerId) return;

    this.#flushDragPending(editor);
    if (session.nativeDragCandidate && !session.nativeDragStarted) {
      editor.enqueue({ type: 'selection', op: { type: 'set_at', page: session.down.page, x: session.down.x, y: session.down.y } });
    }
    this.#session = null;
  }

  cancelPointer(pointerId: number): void {
    if (this.#session?.pointerId !== pointerId) return;
    this.#dragPending = null;
    this.#session = null;
  }

  markNativeDragStarted(): void {
    if (this.#session?.nativeDragCandidate) {
      this.#session.nativeDragStarted = true;
    }
  }

  #flushDragPending(editor: Editor): void {
    const point = this.#dragPending;
    this.#dragPending = null;
    if (!point || !this.#session?.anchor) return;

    const { down } = this.#session;
    const dx = point.x - down.x;
    const dy = point.y - down.y;
    if (!this.#session.dragging && point.page === down.page && dx * dx + dy * dy < DRAG_START_THRESHOLD_PX * DRAG_START_THRESHOLD_PX) {
      return;
    }

    this.#session.dragging = true;
    editor.enqueue({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: this.#session.anchor,
        head_page: point.page,
        head_x: point.x,
        head_y: point.y,
        base_selection: this.#session.baseSelection,
      },
    });
  }
}
