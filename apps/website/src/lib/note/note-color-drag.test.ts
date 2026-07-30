import { afterEach, describe, expect, it, vi } from 'vitest';
import { createNoteColorDrag } from './note-color-drag';

const pointerEvent = (
  type: string,
  {
    pointerId = 1,
    button = 0,
    isPrimary = true,
    clientX = 0,
    clientY = 0,
  }: { pointerId?: number; button?: number; isPrimary?: boolean; clientX?: number; clientY?: number } = {},
): PointerEvent => {
  const event = new MouseEvent(type, { bubbles: true, button, clientX, clientY });
  Object.defineProperties(event, {
    pointerId: { value: pointerId },
    isPrimary: { value: isPrimary },
  });
  return event as PointerEvent;
};

const createHarness = () => {
  const container = document.createElement('div');
  const red = document.createElement('button');
  const redInner = document.createElement('span');
  const blue = document.createElement('button');
  red.dataset.noteColorValue = 'red';
  blue.dataset.noteColorValue = 'blue';
  red.append(redInner);
  container.append(red, blue);
  document.body.append(container);

  let capturedPointerId: number | null = null;
  container.setPointerCapture = vi.fn((pointerId) => {
    capturedPointerId = pointerId;
  });
  container.hasPointerCapture = vi.fn((pointerId) => capturedPointerId === pointerId);
  container.releasePointerCapture = vi.fn((pointerId) => {
    if (capturedPointerId !== pointerId) return;
    capturedPointerId = null;
    container.dispatchEvent(pointerEvent('lostpointercapture', { pointerId }));
  });
  Object.defineProperty(document, 'elementFromPoint', {
    configurable: true,
    value: vi.fn<(x: number, y: number) => Element | null>(() => null),
  });

  const onchange = vi.fn();
  const drag = createNoteColorDrag(container, { onchange });
  return { container, red, redInner, blue, onchange, drag };
};

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe('createNoteColorDrag', () => {
  it('starts only from a primary color button, emits immediately, and captures on the container', () => {
    const { container, redInner, onchange, drag } = createHarness();

    redInner.dispatchEvent(pointerEvent('pointerdown', { pointerId: 7 }));

    expect(onchange.mock.calls).toEqual([['red']]);
    expect(container.setPointerCapture).toHaveBeenCalledWith(7);
    drag.destroy();
  });

  it('ignores secondary, non-primary, and non-color starts', () => {
    const { container, red, onchange, drag } = createHarness();

    red.dispatchEvent(pointerEvent('pointerdown', { button: 2 }));
    red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 2, isPrimary: false }));
    container.dispatchEvent(pointerEvent('pointerdown'));

    expect(onchange).not.toHaveBeenCalled();
    expect(container.setPointerCapture).not.toHaveBeenCalled();
    drag.destroy();
  });

  it('uses the owner pointer and elementFromPoint to emit each newly crossed in-container color once', () => {
    const { container, red, blue, onchange, drag } = createHarness();
    const outside = document.createElement('button');
    outside.dataset.noteColorValue = 'outside';
    document.body.append(outside);
    vi.mocked(document.elementFromPoint).mockReturnValue(blue);

    red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 1 }));
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 2, clientX: 10, clientY: 20 }));
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 1, clientX: 10, clientY: 20 }));
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 1, clientX: 11, clientY: 21 }));
    vi.mocked(document.elementFromPoint).mockReturnValue(outside);
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 1, clientX: 30, clientY: 40 }));

    expect(document.elementFromPoint).toHaveBeenCalledWith(10, 20);
    expect(onchange.mock.calls).toEqual([['red'], ['blue']]);
    drag.destroy();
  });

  it.each(['pointerup', 'pointercancel'] as const)('clears and releases the owner on %s', (type) => {
    const { container, red, blue, onchange, drag } = createHarness();
    vi.mocked(document.elementFromPoint).mockReturnValue(blue);
    red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 4 }));

    container.dispatchEvent(pointerEvent(type, { pointerId: 4 }));
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 4 }));

    expect(container.releasePointerCapture).toHaveBeenCalledOnce();
    expect(container.releasePointerCapture).toHaveBeenCalledWith(4);
    expect(onchange.mock.calls).toEqual([['red']]);
    drag.destroy();
  });

  it('ignores non-owner pointerup and pointercancel while the owner remains active', () => {
    const { container, red, blue, onchange, drag } = createHarness();
    vi.mocked(document.elementFromPoint).mockReturnValue(blue);
    red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 4 }));

    container.dispatchEvent(pointerEvent('pointerup', { pointerId: 9, isPrimary: false }));
    container.dispatchEvent(pointerEvent('pointercancel', { pointerId: 9, isPrimary: false }));
    container.dispatchEvent(pointerEvent('pointermove', { pointerId: 4 }));

    expect(container.releasePointerCapture).not.toHaveBeenCalled();
    expect(onchange.mock.calls).toEqual([['red'], ['blue']]);
    drag.destroy();
  });

  it('does not recursively release after capture is lost and releases an active owner on destroy', () => {
    const first = createHarness();
    vi.mocked(document.elementFromPoint).mockReturnValue(first.blue);
    first.red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 3 }));
    first.container.dispatchEvent(pointerEvent('lostpointercapture', { pointerId: 3 }));
    first.container.dispatchEvent(pointerEvent('pointermove', { pointerId: 3 }));
    first.drag.destroy();
    expect(first.container.releasePointerCapture).not.toHaveBeenCalled();
    expect(first.onchange.mock.calls).toEqual([['red']]);

    const second = createHarness();
    second.red.dispatchEvent(pointerEvent('pointerdown', { pointerId: 8 }));
    second.drag.destroy();
    expect(second.container.releasePointerCapture).toHaveBeenCalledOnce();
    expect(second.container.releasePointerCapture).toHaveBeenCalledWith(8);
  });

  it('does not suppress ordinary pointer or keyboard click activation', () => {
    const { red, drag } = createHarness();
    const click = vi.fn((event: MouseEvent) => expect(event.defaultPrevented).toBe(false));
    red.addEventListener('click', click);

    red.dispatchEvent(pointerEvent('pointerdown'));
    red.dispatchEvent(pointerEvent('pointerup'));
    red.click();
    red.click();

    expect(click).toHaveBeenCalledTimes(2);
    drag.destroy();
  });
});
