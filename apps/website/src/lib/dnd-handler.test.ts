import { createDndHandler } from '@typie/ui/utils';
import { afterEach, describe, expect, it, vi } from 'vitest';

const pointerEvent = (type: string, pointerId = 1, clientX = 0) => {
  const event = new MouseEvent(type, { bubbles: true, button: 0, clientX });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: pointerId === 1 },
  });
  return event as PointerEvent;
};

const installPointerCapture = (element: HTMLElement) => {
  let capturedPointerId: number | null = null;

  element.setPointerCapture = vi.fn((pointerId) => {
    capturedPointerId = pointerId;
  });
  element.hasPointerCapture = vi.fn((pointerId) => capturedPointerId === pointerId);
  element.releasePointerCapture = vi.fn((pointerId) => {
    if (capturedPointerId !== pointerId) return;
    capturedPointerId = null;
    element.dispatchEvent(pointerEvent('lostpointercapture', pointerId));
  });

  return {
    lose(pointerId: number) {
      capturedPointerId = null;
      element.dispatchEvent(pointerEvent('lostpointercapture', pointerId));
    },
  };
};

const installAnimationFrames = () => {
  const callbacks: FrameRequestCallback[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    callbacks.push(callback);
    return callbacks.length;
  });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());

  return {
    flush() {
      const current = [...callbacks];
      callbacks.length = 0;
      for (const callback of current) callback(0);
    },
  };
};

const createElement = () => {
  const element = document.createElement('div');
  document.body.append(element);
  return element;
};

afterEach(() => {
  document.body.replaceChildren();
  document.body.style.cursor = '';
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('createDndHandler', () => {
  it('routes move and end only from the owner pointer', () => {
    const element = createElement();
    installPointerCapture(element);
    const frames = installAnimationFrames();
    const onDragMove = vi.fn();
    const onDragEnd = vi.fn();
    const handler = createDndHandler(element, { threshold: 0, showGhost: false, onDragMove, onDragEnd });

    element.dispatchEvent(pointerEvent('pointerdown', 1));
    const ownerMove = pointerEvent('pointermove', 1, 10);
    element.dispatchEvent(ownerMove);
    frames.flush();

    element.dispatchEvent(pointerEvent('pointermove', 2, 20));
    frames.flush();
    element.dispatchEvent(pointerEvent('pointerup', 2, 20));

    expect(onDragMove).toHaveBeenCalledOnce();
    expect(onDragMove).toHaveBeenCalledWith(ownerMove);
    expect(onDragEnd).not.toHaveBeenCalled();

    const ownerUp = pointerEvent('pointerup', 1, 10);
    element.dispatchEvent(ownerUp);
    expect(onDragEnd).toHaveBeenCalledOnce();
    expect(onDragEnd).toHaveBeenCalledWith(ownerUp);
    handler.destroy();
  });

  it('cancels an active drag when pointer capture is lost', () => {
    const element = createElement();
    const capture = installPointerCapture(element);
    const frames = installAnimationFrames();
    const onDragCancel = vi.fn();
    const handler = createDndHandler(element, { threshold: 0, showGhost: false, onDragCancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove', 1, 10));
    frames.flush();
    capture.lose(1);

    expect(onDragCancel).toHaveBeenCalledOnce();
    expect(handler.state().isDragging).toBe(false);
    handler.destroy();
    expect(onDragCancel).toHaveBeenCalledOnce();
  });

  it('cancels consumer state when destroyed during an active drag', () => {
    const element = createElement();
    installPointerCapture(element);
    const frames = installAnimationFrames();
    const onDragCancel = vi.fn();
    const handler = createDndHandler(element, { threshold: 0, showGhost: false, onDragCancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove', 1, 10));
    frames.flush();
    handler.destroy();

    expect(onDragCancel).toHaveBeenCalledOnce();
    expect(handler.state().isDragging).toBe(false);
  });
});
