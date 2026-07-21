import { afterEach, describe, expect, it, vi } from 'vitest';
import { dragPane } from '../routes/website/(dashboard)/[slug]/@pane/dnd';
import type { PaneGroup } from '../routes/website/(dashboard)/[slug]/@pane/context.svelte';

const pointerEvent = ({
  type,
  pointerId = 1,
  clientX = 0,
  button = 0,
  isPrimary = true,
}: {
  type: string;
  pointerId?: number;
  clientX?: number;
  button?: number;
  isPrimary?: boolean;
}) => {
  const event = new MouseEvent(type, { bubbles: true, button, clientX });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: isPrimary },
  });
  return event as PointerEvent;
};

const createElement = () => {
  const element = document.createElement('div');
  let capturedPointerId: number | null = null;
  element.setPointerCapture = vi.fn((pointerId) => {
    capturedPointerId = pointerId;
  });
  element.hasPointerCapture = vi.fn((pointerId) => capturedPointerId === pointerId);
  element.releasePointerCapture = vi.fn((pointerId) => {
    if (capturedPointerId !== pointerId) return;
    capturedPointerId = null;
    element.dispatchEvent(pointerEvent({ type: 'lostpointercapture', pointerId }));
  });
  document.body.append(element);
  return element;
};

const createPaneGroup = () =>
  ({
    draggingPaneId: null,
    updateActiveZone: vi.fn(),
    executeDrop: vi.fn(),
    cancelDrag: vi.fn(),
  }) as unknown as PaneGroup & {
    updateActiveZone: ReturnType<typeof vi.fn>;
    executeDrop: ReturnType<typeof vi.fn>;
    cancelDrag: ReturnType<typeof vi.fn>;
  };

afterEach(() => {
  document.body.replaceChildren();
  document.body.style.cursor = '';
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('dragPane', () => {
  it('ignores foreign pointer termination and drops only for the owner', () => {
    const element = createElement();
    const paneGroup = createPaneGroup();
    const action = dragPane(element, { paneGroup, paneId: 'pane-1' });

    element.dispatchEvent(pointerEvent({ type: 'pointerdown', pointerId: 1 }));
    element.dispatchEvent(pointerEvent({ type: 'pointermove', pointerId: 1, clientX: 20 }));
    element.dispatchEvent(pointerEvent({ type: 'pointerup', pointerId: 2, clientX: 20, isPrimary: false }));

    expect(paneGroup.executeDrop).not.toHaveBeenCalled();
    expect(paneGroup.draggingPaneId).toBe('pane-1');

    element.dispatchEvent(pointerEvent({ type: 'pointerup', pointerId: 1, clientX: 20 }));
    expect(paneGroup.executeDrop).toHaveBeenCalledOnce();
    expect(paneGroup.cancelDrag).not.toHaveBeenCalled();
    action?.destroy?.();
  });

  it('cancels external drag state when destroyed during a drag', () => {
    const element = createElement();
    const paneGroup = createPaneGroup();
    const action = dragPane(element, { paneGroup, paneId: 'pane-1' });

    element.dispatchEvent(pointerEvent({ type: 'pointerdown' }));
    element.dispatchEvent(pointerEvent({ type: 'pointermove', clientX: 20 }));
    action?.destroy?.();

    expect(paneGroup.cancelDrag).toHaveBeenCalledOnce();
    expect(paneGroup.draggingPaneId).toBeNull();
    expect(document.body.style.cursor).toBe('');
  });

  it('does not start from a non-primary or non-left pointer', () => {
    const element = createElement();
    const paneGroup = createPaneGroup();
    const action = dragPane(element, { paneGroup, paneId: 'pane-1' });

    element.dispatchEvent(pointerEvent({ type: 'pointerdown', button: 2 }));
    element.dispatchEvent(pointerEvent({ type: 'pointerdown', pointerId: 2, isPrimary: false }));

    expect(element.setPointerCapture).not.toHaveBeenCalled();
    action?.destroy?.();
  });
});
