import type { InteractiveHit, Rect } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

const pointInRect = (x: number, y: number, r: Rect): boolean => x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height;

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
  const modifiers = { shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey };

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

  PointerState.of(editor).markPointerDown(e.pointerId, !nativeDragCandidate);
  editor.enqueue({ type: 'pointer', event: { type: 'down', page, x, y, count, modifiers } });
  editor.flush();
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
  PointerState.of(editor).enqueueMoveThrottled(editor, local.page, local.x, local.y);
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
  editor.enqueue({ type: 'pointer', event: { type: 'up' } });
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
  editor.enqueue({ type: 'pointer', event: { type: 'cancel' } });
  editor.flush();
  editor.endNativeDragAdmission({ restoreFocus: false });
};

class PointerState {
  static #instances = new WeakMap<Editor, PointerState>();

  #clickTime = 0;
  #clickX = 0;
  #clickY = 0;
  #clickCount = 0;

  #dragPending: { page: number; x: number; y: number } | null = null;
  #dragScheduled = false;
  #activePointerId: number | null = null;
  #capturedPointerId: number | null = null;

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

  enqueueMoveThrottled(editor: Editor, page: number, x: number, y: number) {
    this.#dragPending = { page, x, y };

    if (!this.#dragScheduled) {
      this.#dragScheduled = true;
      requestAnimationFrame(() => {
        this.#dragScheduled = false;
        if (this.#dragPending) {
          const { page, x, y } = this.#dragPending;
          this.#dragPending = null;
          editor.enqueue({ type: 'pointer', event: { type: 'move', page, x, y } });
        }
      });
    }
  }

  markPointerDown(pointerId: number, captured: boolean) {
    this.#activePointerId = pointerId;
    this.#capturedPointerId = captured ? pointerId : null;
  }

  hasActivePointer(pointerId: number): boolean {
    return this.#activePointerId === pointerId;
  }

  releasePointer(target: HTMLElement, pointerId: number): void {
    if (this.#capturedPointerId === pointerId && target.hasPointerCapture(pointerId)) {
      target.releasePointerCapture(pointerId);
    }
    this.#activePointerId = null;
    this.#capturedPointerId = null;
  }
}
