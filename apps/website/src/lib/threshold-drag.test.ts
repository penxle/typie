import { thresholdDrag } from '@typie/ui/actions';
import { afterEach, describe, expect, it, vi } from 'vitest';

const pointerEvent = (type: string, pointerId = 1, clientX = 0, clientY = 0) => {
  const event = new MouseEvent(type, { bubbles: true, button: 0, clientX, clientY });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: pointerId === 1 },
  });
  return event as PointerEvent;
};

const createElement = ({ captureFails = false }: { captureFails?: boolean } = {}) => {
  const element = document.createElement('div');
  let capturedPointerId: number | null = null;

  element.setPointerCapture = vi.fn((pointerId) => {
    if (captureFails) throw new Error('capture failed');
    capturedPointerId = pointerId;
  });
  element.hasPointerCapture = vi.fn((pointerId) => capturedPointerId === pointerId);
  element.releasePointerCapture = vi.fn((pointerId) => {
    if (capturedPointerId !== pointerId) return;
    capturedPointerId = null;
  });
  document.body.append(element);

  return {
    element,
    lose(pointerId = 1) {
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
    take() {
      return callbacks.shift();
    },
  };
};

afterEach(() => {
  document.body.replaceChildren();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('thresholdDrag', () => {
  it('treats a normal pointerup below the threshold as a press', () => {
    const { element } = createElement();
    installAnimationFrames();
    const press = vi.fn();
    const end = vi.fn();
    const cancel = vi.fn();
    const action = thresholdDrag(element, {
      threshold: 5,
      start: () => ({ id: 1 }),
      activate: () => true,
      press,
      end,
      cancel,
    });

    element.dispatchEvent(pointerEvent('pointerdown'));
    const up = pointerEvent('pointerup', 1, 3);
    element.dispatchEvent(up);

    expect(press).toHaveBeenCalledOnce();
    expect(press).toHaveBeenCalledWith({ id: 1 }, up);
    expect(end).not.toHaveBeenCalled();
    expect(cancel).not.toHaveBeenCalled();
    action.destroy();
  });

  it('activates once and immediately delivers the threshold-crossing move', () => {
    const { element } = createElement();
    const frames = installAnimationFrames();
    const activate = vi.fn(() => true);
    const move = vi.fn();
    const action = thresholdDrag(element, {
      threshold: 5,
      start: () => ({ id: 1 }),
      activate,
      move,
    });

    const down = pointerEvent('pointerdown', 1, 0, 0);
    const crossingMove = pointerEvent('pointermove', 1, 6, 0);
    element.dispatchEvent(down);
    element.dispatchEvent(crossingMove);
    frames.flush();
    element.dispatchEvent(pointerEvent('pointermove', 1, 10, 0));
    frames.flush();

    expect(activate).toHaveBeenCalledOnce();
    expect(activate).toHaveBeenCalledWith({ id: 1 }, down, crossingMove, expect.any(Object));
    expect(move).toHaveBeenNthCalledWith(1, { id: 1 }, crossingMove);
    expect(move).toHaveBeenCalledTimes(2);
    expect(action.state()).toEqual({ active: true });
    action.destroy();
  });

  it('silently terminates a rejected activation', () => {
    const { element } = createElement();
    const frames = installAnimationFrames();
    const press = vi.fn();
    const end = vi.fn();
    const cancel = vi.fn();
    const action = thresholdDrag(element, {
      threshold: 5,
      start: () => ({ id: 1 }),
      activate: () => false,
      press,
      end,
      cancel,
    });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove', 1, 6));
    frames.flush();
    element.dispatchEvent(pointerEvent('pointerup', 1, 6));

    expect(press).not.toHaveBeenCalled();
    expect(end).not.toHaveBeenCalled();
    expect(cancel).not.toHaveBeenCalled();
    expect(action.state()).toEqual({ active: false });
    action.destroy();
  });

  it('flushes the latest queued move before ending', () => {
    const { element } = createElement();
    const frames = installAnimationFrames();
    const calls: string[] = [];
    const move = vi.fn(() => {
      calls.push('move');
    });
    const end = vi.fn(() => {
      calls.push('end');
    });
    const action = thresholdDrag(element, {
      threshold: 0,
      start: () => ({ id: 1 }),
      activate: () => true,
      move,
      end,
    });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove', 1, 5));
    frames.flush();
    const finalMove = pointerEvent('pointermove', 1, 10);
    element.dispatchEvent(finalMove);
    element.dispatchEvent(pointerEvent('pointerup', 1, 10));

    expect(move).toHaveBeenLastCalledWith({ id: 1 }, finalMove);
    expect(calls).toEqual(['move', 'move', 'end']);
    action.destroy();
  });

  it.each([
    ['pointer cancellation', 'pointercancel'],
    ['lost capture', 'lostpointercapture'],
  ] as const)('does not turn %s into a press', (_label, eventType) => {
    const { element } = createElement();
    installAnimationFrames();
    const press = vi.fn();
    const cancel = vi.fn();
    const action = thresholdDrag(element, {
      start: () => ({ id: 1 }),
      activate: () => true,
      press,
      cancel,
    });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent(eventType));

    expect(press).not.toHaveBeenCalled();
    expect(cancel).toHaveBeenCalledWith({ id: 1 }, eventType, false, expect.any(MouseEvent));
    action.destroy();
  });

  it('reports capture failure, programmatic cancellation, and destruction without pressing', () => {
    const press = vi.fn();
    const cancel = vi.fn();

    const captureFailure = createElement({ captureFails: true }).element;
    const failedAction = thresholdDrag(captureFailure, {
      start: () => ({ id: 1 }),
      activate: () => true,
      press,
      cancel,
    });
    captureFailure.dispatchEvent(pointerEvent('pointerdown'));
    expect(cancel).toHaveBeenLastCalledWith({ id: 1 }, 'capture-failed', false, expect.any(MouseEvent));

    const programmatic = createElement().element;
    const programmaticAction = thresholdDrag(programmatic, {
      start: () => ({ id: 2 }),
      activate: () => true,
      press,
      cancel,
    });
    programmatic.dispatchEvent(pointerEvent('pointerdown'));
    programmaticAction.cancel();
    expect(cancel).toHaveBeenLastCalledWith({ id: 2 }, 'programmatic', false, undefined);

    const destroyed = createElement().element;
    const destroyedAction = thresholdDrag(destroyed, {
      start: () => ({ id: 3 }),
      activate: () => true,
      press,
      cancel,
    });
    destroyed.dispatchEvent(pointerEvent('pointerdown'));
    destroyedAction.destroy();
    expect(cancel).toHaveBeenLastCalledWith({ id: 3 }, 'destroy', false, undefined);
    expect(press).not.toHaveBeenCalled();

    failedAction.destroy();
    programmaticAction.destroy();
  });

  it('reports active cancellation exactly once', () => {
    const { element, lose } = createElement();
    const frames = installAnimationFrames();
    const cancel = vi.fn();
    const action = thresholdDrag(element, {
      threshold: 0,
      start: () => ({ id: 1 }),
      activate: () => true,
      cancel,
    });

    element.dispatchEvent(pointerEvent('pointerdown'));
    element.dispatchEvent(pointerEvent('pointermove', 1, 10));
    frames.flush();
    lose();
    action.destroy();

    expect(cancel).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledWith({ id: 1 }, 'lostpointercapture', true, expect.any(MouseEvent));
  });

  it('keeps old controls and animation frames scoped to their session', () => {
    const { element } = createElement();
    const frames = installAnimationFrames();
    const controls: { cancel: () => void }[] = [];
    const move = vi.fn();
    const cancel = vi.fn();
    const action = thresholdDrag(element, {
      threshold: 0,
      start: (event) => ({ id: event.pointerId }),
      activate: (_session, _startEvent, _event, currentControls) => {
        controls.push(currentControls);
        return true;
      },
      move,
      cancel,
    });

    element.dispatchEvent(pointerEvent('pointerdown', 1));
    element.dispatchEvent(pointerEvent('pointermove', 1, 10));
    const oldFrame = frames.take();
    oldFrame?.(0);
    element.dispatchEvent(pointerEvent('pointerup', 1, 10));

    element.dispatchEvent(pointerEvent('pointerdown', 1));
    element.dispatchEvent(pointerEvent('pointermove', 1, 20));
    frames.flush();
    const callsBeforeStaleWork = move.mock.calls.length;
    controls[0]?.cancel();
    oldFrame?.(0);

    expect(action.state()).toEqual({ active: true });
    expect(move).toHaveBeenCalledTimes(callsBeforeStaleWork);
    expect(cancel).not.toHaveBeenCalled();
    action.destroy();
  });
});
