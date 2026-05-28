import { describe, expect, it, vi } from 'vitest';
import { handlePointerDown, handlePointerUp } from './pointer';
import type { Editor } from '../editor.svelte';

const createPointerTarget = ({ captured = false } = {}) => {
  return {
    tabIndex: 0,
    hasAttribute: vi.fn(() => true),
    setPointerCapture: vi.fn(),
    releasePointerCapture: vi.fn(),
    hasPointerCapture: vi.fn(() => captured),
    removeAttribute: vi.fn(),
    setAttribute: vi.fn(),
  };
};

const createPointerEvent = ({
  pointerId = 1,
  button = 0,
  timeStamp = 1000,
  target = createPointerTarget(),
}: {
  pointerId?: number;
  button?: number;
  timeStamp?: number;
  target?: ReturnType<typeof createPointerTarget>;
} = {}) => {
  return {
    pointerId,
    pointerType: 'mouse',
    button,
    buttons: button === 0 ? 1 : 0,
    clientX: 110,
    clientY: 220,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    timeStamp,
    currentTarget: target,
    preventDefault: vi.fn(),
  } as unknown as PointerEvent & { currentTarget: HTMLElement };
};

const createEditor = ({
  selectionHit = false,
  isSelectionCollapsed = true,
}: {
  selectionHit?: boolean;
  isSelectionCollapsed?: boolean;
} = {}) => {
  return {
    readOnly: false,
    isSelectionCollapsed,
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    interactiveHitTest: vi.fn(() => null),
    selectionHitTest: vi.fn(() => selectionHit),
    beginNativeDragAdmission: vi.fn(),
    endNativeDragAdmission: vi.fn(),
    enqueue: vi.fn(),
    flush: vi.fn(),
    gesture: {
      handlePointerDown: vi.fn(),
      handlePointerMove: vi.fn(),
      handlePointerUp: vi.fn(),
      handlePointerCancel: vi.fn(),
    },
    updatePointerHover: vi.fn(),
  } as unknown as Editor & {
    enqueue: ReturnType<typeof vi.fn>;
    flush: ReturnType<typeof vi.fn>;
    selectionHitTest: ReturnType<typeof vi.fn>;
  };
};

describe('pointer native drag admission', () => {
  it('sends pointer down immediately on a selected range without capturing pointer and temporarily removes tabindex', () => {
    vi.useFakeTimers();
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false });
    const target = createPointerTarget();
    const down = createPointerEvent({ target });

    handlePointerDown(editor, down);

    expect(target.setPointerCapture).not.toHaveBeenCalled();
    expect(editor.beginNativeDragAdmission).toHaveBeenCalledTimes(1);
    expect(target.removeAttribute).toHaveBeenCalledWith('tabindex');
    expect(target.setAttribute).not.toHaveBeenCalled();
    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'pointer',
      event: {
        type: 'down',
        page: 0,
        x: 10,
        y: 20,
        count: 1,
        modifiers: { shift: false, ctrl: false, alt: false, meta: false },
      },
    });
    expect(editor.flush).toHaveBeenCalledTimes(1);
    vi.runAllTimers();
    expect(target.setAttribute).toHaveBeenCalledWith('tabindex', '0');
    vi.useRealTimers();

    handlePointerUp(editor, createPointerEvent({ target }));

    expect(editor.endNativeDragAdmission).toHaveBeenCalledWith({ restoreFocus: true });
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'pointer', event: { type: 'up' } });
    expect(editor.flush).toHaveBeenCalledTimes(2);
  });

  it('restores tabindex even after the browser clears event.currentTarget', () => {
    vi.useFakeTimers();
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false });
    const target = createPointerTarget();
    let currentTarget: ReturnType<typeof createPointerTarget> | null = target;
    const down = {
      ...createPointerEvent({ target }),
      get currentTarget() {
        return currentTarget;
      },
    } as unknown as PointerEvent & { currentTarget: HTMLElement };

    handlePointerDown(editor, down);
    currentTarget = null;

    expect(editor.beginNativeDragAdmission).toHaveBeenCalledTimes(1);
    expect(() => vi.runAllTimers()).not.toThrow();
    expect(target.setAttribute).toHaveBeenCalledWith('tabindex', '0');
    vi.useRealTimers();
  });

  it('captures regular primary down and sends up even if capture is already gone', () => {
    const editor = createEditor();
    const target = createPointerTarget({ captured: true });

    handlePointerDown(editor, createPointerEvent({ target }));
    handlePointerUp(editor, createPointerEvent({ target }));

    expect(editor.beginNativeDragAdmission).not.toHaveBeenCalled();
    expect(target.setPointerCapture).toHaveBeenCalledWith(1);
    expect(target.releasePointerCapture).toHaveBeenCalledWith(1);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'pointer', event: { type: 'up' } });
  });
});
