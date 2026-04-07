import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

export const handlePointerDown: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  const rect = e.currentTarget.getBoundingClientRect();
  const global = { x: e.clientX - rect.x, y: e.clientY - rect.y };
  const local = editor.globalToLocal(global.x, global.y);
  if (!local) {
    return;
  }

  const { page, x, y } = local;
  const count = PointerState.of(editor).resolveClickCount(e);
  const modifiers = { shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey };

  e.currentTarget.setPointerCapture(e.pointerId);
  editor.enqueue({ type: 'pointer', event: { type: 'down', page, x, y, count, modifiers } });
};

export const handlePointerMove: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  if (!e.currentTarget.hasPointerCapture(e.pointerId)) {
    return;
  }

  const rect = e.currentTarget.getBoundingClientRect();
  const global = { x: e.clientX - rect.x, y: e.clientY - rect.y };
  const local = editor.globalToLocal(global.x, global.y);
  if (!local) {
    return;
  }

  e.preventDefault();
  PointerState.of(editor).enqueueMoveThrottled(editor, local.page, local.x, local.y);
};

export const handlePointerUp: EditorEventHandler<HTMLElement, PointerEvent> = (editor) => {
  editor.enqueue({ type: 'pointer', event: { type: 'up' } });
};

class PointerState {
  static #instances = new WeakMap<Editor, PointerState>();

  #clickTime = 0;
  #clickX = 0;
  #clickY = 0;
  #clickCount = 0;

  #dragPending: { page: number; x: number; y: number } | null = null;
  #dragScheduled = false;

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
}
