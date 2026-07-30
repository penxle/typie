import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteEdits } from '$lib/note/note-edit-state.svelte';
import NoteDragTestHost from './note-drag-test-host.svelte';

const pointerEvent = (type: string, pointerId: number, clientY: number) => {
  const event = new MouseEvent(type, { bubbles: true, button: 0, clientY });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: true },
  });
  return event as PointerEvent;
};

describe('global note dragging', () => {
  const animationFrames: FrameRequestCallback[] = [];
  let capturedPointerId: number | null = null;

  beforeEach(() => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {
          return null;
        }
        disconnect() {
          return null;
        }
      },
    );
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      animationFrames.push(callback);
      return animationFrames.length;
    });
    vi.stubGlobal('cancelAnimationFrame', vi.fn());
  });

  const installPointerCapture = (element: HTMLElement) => {
    element.setPointerCapture = vi.fn((pointerId) => {
      capturedPointerId = pointerId;
    });
    element.hasPointerCapture = vi.fn((pointerId) => capturedPointerId === pointerId);
    element.releasePointerCapture = vi.fn((pointerId) => {
      if (capturedPointerId === pointerId) capturedPointerId = null;
    });
  };

  afterEach(() => {
    animationFrames.length = 0;
    capturedPointerId = null;
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  const flushAnimationFrames = () => {
    const callbacks = [...animationFrames];
    animationFrames.length = 0;
    for (const callback of callbacks) callback(0);
  };

  it('does not let the previous drag end cancel the next pending attempt', async () => {
    const noteEdits = new NoteEdits({
      isTerminallyDeleted: () => false,
      save: async () => ({ kind: 'saved', snapshot: { content: 'server content', color: 'gray' } }),
    });
    const ondragstart = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteDragTestHost, { target, props: { noteEdits, ondragstart } });

    try {
      await tick();
      const note = target.querySelector<HTMLElement>('[data-note-id="note-1"]');
      expect(note).not.toBeNull();
      if (!note) return;
      installPointerCapture(document.documentElement);

      note.dispatchEvent(pointerEvent('pointerdown', 1, 100));
      note.dispatchEvent(pointerEvent('pointermove', 1, 110));
      flushAnimationFrames();
      note.dispatchEvent(pointerEvent('pointerup', 1, 110));
      expect(ondragstart).toHaveBeenCalledOnce();

      note.dispatchEvent(pointerEvent('pointerdown', 2, 100));
      await tick();
      note.dispatchEvent(pointerEvent('pointermove', 2, 110));
      flushAnimationFrames();

      expect(ondragstart).toHaveBeenCalledTimes(2);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps the drag session on a stable capture target when the card moves in the DOM', async () => {
    const noteEdits = new NoteEdits({
      isTerminallyDeleted: () => false,
      save: async () => ({ kind: 'saved', snapshot: { content: 'server content', color: 'gray' } }),
    });
    const ondragstart = vi.fn();
    const ondragend = vi.fn();
    const ondragcancel = vi.fn();
    const target = document.createElement('div');
    const movedParent = document.createElement('div');
    document.body.append(target, movedParent);
    const component = mount(NoteDragTestHost, {
      target,
      props: { noteEdits, ondragstart, ondragend, ondragcancel },
    });

    try {
      await tick();
      const note = target.querySelector<HTMLElement>('[data-note-id="note-1"]');
      expect(note).not.toBeNull();
      if (!note) return;
      installPointerCapture(note);
      installPointerCapture(document.documentElement);
      const bodyChildCount = document.body.childElementCount;
      const headStyleCount = document.head.querySelectorAll('style').length;

      note.dispatchEvent(pointerEvent('pointerdown', 1, 100));
      note.dispatchEvent(pointerEvent('pointermove', 1, 110));
      flushAnimationFrames();
      expect(ondragstart).toHaveBeenCalledOnce();
      expect(document.body.childElementCount).toBe(bodyChildCount + 1);
      expect(document.head.querySelectorAll('style').length).toBe(headStyleCount + 1);

      movedParent.append(note);
      document.documentElement.dispatchEvent(pointerEvent('pointermove', 1, 120));
      flushAnimationFrames();
      document.documentElement.dispatchEvent(pointerEvent('pointerup', 1, 120));

      expect(ondragcancel).not.toHaveBeenCalled();
      expect(ondragend).toHaveBeenCalledOnce();
      expect(document.body.childElementCount).toBe(bodyChildCount);
      expect(document.head.querySelectorAll('style').length).toBe(headStyleCount);
    } finally {
      await unmount(component);
      target.remove();
      movedParent.remove();
    }
  });
});
