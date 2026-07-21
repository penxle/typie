import { pointerCapture } from '@typie/ui/actions';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PointerCaptureParameters } from '@typie/ui/actions';

const pointerEvent = (type: string, pointerId = 1) => {
  const event = new MouseEvent(type, { bubbles: true, button: 0 });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: true },
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

const createElement = () => {
  const element = document.createElement('div');
  document.body.append(element);
  return element;
};

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe('pointerCapture', () => {
  it('does not capture when start declines the gesture', () => {
    const element = createElement();
    installPointerCapture(element);
    const move = vi.fn();
    const action = pointerCapture(element, { start: () => null, move });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove'));

    expect(element.setPointerCapture).not.toHaveBeenCalled();
    expect(move).not.toHaveBeenCalled();
    action.destroy?.();
  });

  it('routes only the owner pointer and ignores overlapping starts', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const start = vi.fn(() => session);
    const move = vi.fn();
    const end = vi.fn();
    const cancel = vi.fn();
    const action = pointerCapture(element, { start, move, end, cancel });

    element.dispatchEvent(pointerEvent('pointerdown', 1));
    element.dispatchEvent(pointerEvent('pointerdown', 2));
    element.dispatchEvent(pointerEvent('pointermove', 2));
    element.dispatchEvent(pointerEvent('pointerup', 2));
    element.dispatchEvent(pointerEvent('pointercancel', 2));
    element.dispatchEvent(pointerEvent('lostpointercapture', 2));
    const ownerMove = pointerEvent('pointermove', 1);
    element.dispatchEvent(ownerMove);

    expect(start).toHaveBeenCalledTimes(1);
    expect(move).toHaveBeenCalledOnce();
    expect(move).toHaveBeenCalledWith(session, ownerMove);
    expect(end).not.toHaveBeenCalled();
    expect(cancel).not.toHaveBeenCalled();
    action.destroy?.();
  });

  it('ends once and ignores capture loss caused by normal release', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const end = vi.fn();
    const cancel = vi.fn();
    const action = pointerCapture(element, { start: () => session, end, cancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    const up = pointerEvent('pointerup');
    element.dispatchEvent(up);
    element.dispatchEvent(pointerEvent('pointerup'));

    expect(element.releasePointerCapture).toHaveBeenCalledOnce();
    expect(end).toHaveBeenCalledOnce();
    expect(end).toHaveBeenCalledWith(session, up);
    expect(cancel).not.toHaveBeenCalled();
    action.destroy?.();
  });

  it('cancels once on pointer cancellation', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const cancel = vi.fn();
    const action = pointerCapture(element, { start: () => session, cancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    const event = pointerEvent('pointercancel');
    element.dispatchEvent(event);

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith(session, 'pointercancel', event);
    action.destroy?.();
    expect(cancel).toHaveBeenCalledOnce();
  });

  it('cancels when pointer capture is lost unexpectedly', () => {
    const element = createElement();
    const capture = installPointerCapture(element);
    const session = { id: 'owner' };
    const cancel = vi.fn();
    const action = pointerCapture(element, { start: () => session, cancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    capture.lose(1);

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith(session, 'lostpointercapture', expect.any(MouseEvent));
    action.destroy?.();
    expect(cancel).toHaveBeenCalledOnce();
  });

  it('cancels on destroy and removes its listeners', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const start = vi.fn(() => session);
    const cancel = vi.fn();
    const action = pointerCapture(element, { start, cancel });

    element.dispatchEvent(pointerEvent('pointerdown'));
    action.destroy?.();

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith(session, 'destroy', undefined);
    element.dispatchEvent(pointerEvent('pointerdown'));
    expect(start).toHaveBeenCalledOnce();
  });

  it('supports programmatic cancellation without destroying the action', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const start = vi.fn(() => session);
    const cancel = vi.fn();
    const action = pointerCapture(element, { start, cancel }) as ReturnType<typeof pointerCapture> & { cancel?: () => void };

    element.dispatchEvent(pointerEvent('pointerdown'));
    action.cancel?.();

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith(session, 'programmatic', undefined);
    element.dispatchEvent(pointerEvent('pointerdown'));
    expect(start).toHaveBeenCalledTimes(2);
    action.destroy?.();
  });

  it('cancels state changed by start when capture acquisition fails', () => {
    const element = createElement();
    installPointerCapture(element);
    element.setPointerCapture = vi.fn(() => {
      throw new DOMException('capture failed');
    });
    const session = { id: 'owner' };
    const cancel = vi.fn();
    const action = pointerCapture(element, { start: () => session, cancel });

    const down = pointerEvent('pointerdown');
    element.dispatchEvent(down);

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith(session, 'capture-failed', down);
    action.destroy?.();
    expect(cancel).toHaveBeenCalledOnce();
  });

  it('uses updated callbacks without replacing active session data', () => {
    const element = createElement();
    installPointerCapture(element);
    const session = { id: 'owner' };
    const oldMove = vi.fn();
    const newMove = vi.fn();
    const action = pointerCapture(element, { start: () => session, move: oldMove });

    element.dispatchEvent(pointerEvent('pointerdown'));
    action.update?.({ start: () => ({ id: 'replacement' }), move: newMove });
    const move = pointerEvent('pointermove');
    element.dispatchEvent(move);

    expect(oldMove).not.toHaveBeenCalled();
    expect(newMove).toHaveBeenCalledWith(session, move);
    action.destroy?.();
  });

  it('supports rollback policy without committing a cancelled preview', () => {
    const element = createElement();
    installPointerCapture(element);
    let preview = 10;
    let commitCount = 0;
    const parameters: PointerCaptureParameters<number> = {
      start: () => preview,
      move: (_, event) => (preview = event.clientX),
      end: () => commitCount++,
      cancel: (initial) => (preview = initial),
    };
    const action = pointerCapture(element, parameters);

    element.dispatchEvent(pointerEvent('pointerdown'));
    const move = pointerEvent('pointermove');
    Object.defineProperty(move, 'clientX', { value: 20 });
    element.dispatchEvent(move);
    element.dispatchEvent(pointerEvent('pointercancel'));

    expect(preview).toBe(10);
    expect(commitCount).toBe(0);
    action.destroy?.();
  });

  it('finalizes live cancellation but not teardown', () => {
    const element = createElement();
    installPointerCapture(element);
    let value = 10;
    let dragging = false;
    let finalizeCount = 0;
    const parameters: PointerCaptureParameters<true> = {
      start: () => {
        dragging = true;
        return true;
      },
      move: (_, event) => (value = event.clientX),
      end: () => {
        dragging = false;
        finalizeCount++;
      },
      cancel: (_, reason) => {
        dragging = false;
        if (reason !== 'destroy') finalizeCount++;
      },
    };
    const action = pointerCapture(element, parameters);

    element.dispatchEvent(pointerEvent('pointerdown'));
    const move = pointerEvent('pointermove');
    Object.defineProperty(move, 'clientX', { value: 20 });
    element.dispatchEvent(move);
    element.dispatchEvent(pointerEvent('pointercancel'));

    expect(value).toBe(20);
    expect(dragging).toBe(false);
    expect(finalizeCount).toBe(1);

    element.dispatchEvent(pointerEvent('pointerdown'));
    action.destroy?.();

    expect(value).toBe(20);
    expect(dragging).toBe(false);
    expect(finalizeCount).toBe(1);
  });
});
