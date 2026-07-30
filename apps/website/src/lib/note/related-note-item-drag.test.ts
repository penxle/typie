import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteEdits } from '$lib/note/note-edit-state.svelte';
import RelatedNoteItemDragTestHost from './related-note-item-drag-test-host.svelte';

const { createFragment } = vi.hoisted(() => ({
  createFragment: vi.fn((_document: unknown, getKey: () => unknown) => ({ data: getKey() })),
}));

vi.mock('@mearie/svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@mearie/svelte')>()),
  createFragment,
}));

const pointerEvent = (type: string, pointerId: number, clientY: number) => {
  const event = new MouseEvent(type, { bubbles: true, button: 0, clientY });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: true },
  });
  return event as PointerEvent;
};

describe('RelatedNoteItem dragging', () => {
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
    const onDragStart = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(RelatedNoteItemDragTestHost, { target, props: { noteEdits, onDragStart } });

    try {
      await tick();
      const note = target.querySelector<HTMLElement>('[data-related-note-item-id="note-1"]');
      expect(note).not.toBeNull();
      if (!note) return;
      installPointerCapture(document.documentElement);

      note.dispatchEvent(pointerEvent('pointerdown', 1, 100));
      note.dispatchEvent(pointerEvent('pointermove', 1, 110));
      flushAnimationFrames();
      note.dispatchEvent(pointerEvent('pointerup', 1, 110));
      expect(onDragStart).toHaveBeenCalledOnce();

      note.dispatchEvent(pointerEvent('pointerdown', 2, 100));
      await tick();
      note.dispatchEvent(pointerEvent('pointermove', 2, 110));
      flushAnimationFrames();

      expect(onDragStart).toHaveBeenCalledTimes(2);
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
