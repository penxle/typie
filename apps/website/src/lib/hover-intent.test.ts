import { hoverIntent } from '@typie/ui/actions';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const pointer = (element: HTMLElement, type: string, x: number, y: number, pointerType = 'mouse') => {
  const event = new MouseEvent(type, { clientX: x, clientY: y });
  Object.defineProperty(event, 'pointerType', { value: pointerType });
  element.dispatchEvent(event);
};

describe('hoverIntent', () => {
  let element: HTMLElement;

  beforeEach(() => {
    vi.useFakeTimers();
    element = document.createElement('div');
    document.body.append(element);
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  it('fires after two consecutive low-speed samples', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(50);
    pointer(element, 'pointermove', 13, 10);
    await vi.advanceTimersByTimeAsync(100);
    pointer(element, 'pointermove', 16, 10);
    await vi.advanceTimersByTimeAsync(49);
    expect(onIntent).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    pointer(element, 'pointermove', 30, 10);
    await vi.advanceTimersByTimeAsync(400);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('supports a single low-speed sample for compact hover targets', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, samples: 1, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(99);
    expect(onIntent).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('fires synchronously when the delay is zero', () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 0, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('resets only the low-speed streak after movement exceeds the sensitivity', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 500, sensitivity: 6, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(100);
    pointer(element, 'pointermove', 20, 10);

    await vi.advanceTimersByTimeAsync(100);
    expect(onIntent).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(199);
    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('uses a custom sensitivity for the early intent threshold', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 500, sensitivity: 2, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    for (const x of [13, 16, 19, 22]) {
      await vi.advanceTimersByTimeAsync(50);
      pointer(element, 'pointermove', x, 10);
      await vi.advanceTimersByTimeAsync(50);
    }

    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(99);
    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('falls back to the configured maximum delay while the pointer keeps moving quickly', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, onIntent });

    pointer(element, 'pointerenter', 0, 0);
    for (const x of [10, 20, 30]) {
      await vi.advanceTimersByTimeAsync(50);
      pointer(element, 'pointermove', x, 0);
      await vi.advanceTimersByTimeAsync(50);
    }

    await vi.advanceTimersByTimeAsync(99);
    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('cancels on leave and starts a new session on re-entry', async () => {
    const onEnter = vi.fn();
    const onIntent = vi.fn();
    const onLeave = vi.fn();
    const action = hoverIntent(element, { delay: 400, onEnter, onIntent, onLeave });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(100);
    pointer(element, 'pointerleave', 10, 10);
    await vi.advanceTimersByTimeAsync(400);

    expect(onEnter).toHaveBeenCalledOnce();
    expect(onLeave).toHaveBeenCalledOnce();
    expect(onIntent).not.toHaveBeenCalled();

    pointer(element, 'pointerenter', 20, 20);
    await vi.advanceTimersByTimeAsync(200);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('ignores touch pointers', async () => {
    const onEnter = vi.fn();
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, onEnter, onIntent });

    pointer(element, 'pointerenter', 10, 10, 'touch');
    await vi.advanceTimersByTimeAsync(400);

    expect(onEnter).not.toHaveBeenCalled();
    expect(onIntent).not.toHaveBeenCalled();

    action?.destroy?.();
  });

  it('does not let touch events interrupt mouse intent detection', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(100);
    pointer(element, 'pointermove', 100, 100, 'touch');
    pointer(element, 'pointercancel', 100, 100, 'touch');

    await vi.advanceTimersByTimeAsync(100);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('restarts active intent detection when the maximum delay changes', async () => {
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 500, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(100);
    action?.update?.({ delay: 200, onIntent });

    await vi.advanceTimersByTimeAsync(199);
    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });

  it('starts intent detection when it becomes enabled during hover', async () => {
    const onEnter = vi.fn();
    const onIntent = vi.fn();
    const action = hoverIntent(element, { delay: 400, intentEnabled: false, onEnter, onIntent });

    pointer(element, 'pointerenter', 10, 10);
    await vi.advanceTimersByTimeAsync(1000);
    expect(onEnter).toHaveBeenCalledOnce();
    expect(onIntent).not.toHaveBeenCalled();

    action?.update?.({ delay: 400, intentEnabled: true, onEnter, onIntent });
    await vi.advanceTimersByTimeAsync(199);
    expect(onIntent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(onIntent).toHaveBeenCalledOnce();

    action?.destroy?.();
  });
});
