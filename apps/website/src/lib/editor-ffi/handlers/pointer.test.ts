import { describe, expect, it, vi } from 'vitest';
import { handlePointerDown, handlePointerMove, handlePointerUp, markNativeSelectionDragStarted } from './pointer';
import type { Editor } from '../editor.svelte';

const collapsedSelection = {
  anchor: { node: 't1', offset: 2, affinity: 'downstream' },
  head: { node: 't1', offset: 2, affinity: 'downstream' },
} as const;

const rangeSelection = {
  anchor: { node: 't1', offset: 1, affinity: 'downstream' },
  head: { node: 't1', offset: 4, affinity: 'downstream' },
} as const;

type PointerTarget = Omit<
  HTMLDivElement,
  'setPointerCapture' | 'releasePointerCapture' | 'hasPointerCapture' | 'removeAttribute' | 'setAttribute'
> & {
  setPointerCapture: ReturnType<typeof vi.fn>;
  releasePointerCapture: ReturnType<typeof vi.fn>;
  hasPointerCapture: ReturnType<typeof vi.fn>;
  removeAttribute: ReturnType<typeof vi.fn>;
  setAttribute: ReturnType<typeof vi.fn>;
};

const createPointerTarget = ({ captured = false } = {}) => {
  const target = document.createElement('div') as PointerTarget;
  target.tabIndex = 0;
  target.setPointerCapture = vi.fn();
  target.releasePointerCapture = vi.fn();
  target.hasPointerCapture = vi.fn(() => captured);
  target.removeAttribute = vi.fn();
  target.setAttribute = vi.fn();
  return target;
};

const createPointerEvent = ({
  pointerId = 1,
  button = 0,
  timeStamp = 1000,
  target = createPointerTarget(),
  clientX = 110,
  clientY = 220,
  shiftKey = false,
}: {
  pointerId?: number;
  button?: number;
  timeStamp?: number;
  target?: ReturnType<typeof createPointerTarget>;
  clientX?: number;
  clientY?: number;
  shiftKey?: boolean;
} = {}) => {
  return {
    pointerId,
    pointerType: 'mouse',
    button,
    buttons: button === 0 ? 1 : 0,
    clientX,
    clientY,
    shiftKey,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    timeStamp,
    target,
    currentTarget: target,
    preventDefault: vi.fn(),
  } as unknown as PointerEvent & { currentTarget: HTMLElement };
};

const createEditor = ({
  selectionHit = false,
  isSelectionCollapsed = true,
  selection = collapsedSelection,
  readOnly = false,
}: {
  selectionHit?: boolean;
  isSelectionCollapsed?: boolean;
  selection?: typeof collapsedSelection | typeof rangeSelection | undefined;
  readOnly?: boolean;
} = {}) => {
  return {
    readOnly,
    isSelectionCollapsed,
    selection,
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    interactiveHitTest: vi.fn(() => null),
    selectionHitTest: vi.fn(() => selectionHit),
    beginNativeDragAdmission: vi.fn(),
    endNativeDragAdmission: vi.fn(),
    enqueue: vi.fn(),
    flush: vi.fn(),
    scrollIntoView: vi.fn(),
    suspendToolbarSync: vi.fn(),
    resumeToolbarSync: vi.fn(),
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
    scrollIntoView: ReturnType<typeof vi.fn>;
    selectionHitTest: ReturnType<typeof vi.fn>;
    suspendToolbarSync: ReturnType<typeof vi.fn>;
    resumeToolbarSync: ReturnType<typeof vi.fn>;
  };
};

describe('pointer native drag admission', () => {
  it('admits native drag on a selected range without capturing pointer and collapses on click release', () => {
    vi.useFakeTimers();
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false, selection: rangeSelection });
    const target = createPointerTarget();
    const down = createPointerEvent({ target });

    handlePointerDown(editor, down);

    expect(target.setPointerCapture).not.toHaveBeenCalled();
    expect(editor.beginNativeDragAdmission).toHaveBeenCalledTimes(1);
    expect(target.removeAttribute).toHaveBeenCalledWith('tabindex');
    expect(target.setAttribute).not.toHaveBeenCalled();
    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(editor.flush).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(target.setAttribute).toHaveBeenCalledWith('tabindex', '0');
    vi.useRealTimers();

    handlePointerUp(editor, createPointerEvent({ target }));

    expect(editor.endNativeDragAdmission).toHaveBeenCalledWith({ restoreFocus: true });
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set_at', page: 0, x: 10, y: 20 } });
    expect(editor.scrollIntoView).toHaveBeenCalledWith({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    expect(editor.flush).toHaveBeenCalledTimes(1);
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

  it('does not collapse on pointer up after native dragstart', () => {
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false, selection: rangeSelection });
    const target = createPointerTarget();

    handlePointerDown(editor, createPointerEvent({ target }));
    markNativeSelectionDragStarted(editor);
    handlePointerUp(editor, createPointerEvent({ target }));

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(editor.scrollIntoView).not.toHaveBeenCalled();
    expect(editor.endNativeDragAdmission).toHaveBeenCalledWith({ restoreFocus: true });
  });

  it('captures regular primary down and sends set_at instead of raw pointer messages', () => {
    const editor = createEditor();
    const target = createPointerTarget({ captured: true });

    handlePointerDown(editor, createPointerEvent({ target }));
    handlePointerUp(editor, createPointerEvent({ target }));

    expect(editor.beginNativeDragAdmission).not.toHaveBeenCalled();
    expect(target.setPointerCapture).toHaveBeenCalledWith(1);
    expect(target.releasePointerCapture).toHaveBeenCalledWith(1);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set_at', page: 0, x: 10, y: 20 } });
    expect(editor.scrollIntoView).toHaveBeenCalledWith({ target: { type: 'current_selection_head' }, mode: 'nearest' });
  });

  it('ignores non-touch pointer down on a selection handle marker', () => {
    const editor = createEditor();
    const target = createPointerTarget();
    target.dataset.selectionHandle = 'from';

    handlePointerDown(editor, createPointerEvent({ target }));

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(target.setPointerCapture).not.toHaveBeenCalled();
    expect(editor.gesture.handlePointerDown).not.toHaveBeenCalled();
  });

  it('suspends toolbar sync during a regular selection interaction and resumes on pointer up', () => {
    const editor = createEditor();
    const target = createPointerTarget({ captured: true });

    handlePointerDown(editor, createPointerEvent({ target }));
    expect(editor.suspendToolbarSync).toHaveBeenCalledTimes(1);
    expect(editor.resumeToolbarSync).not.toHaveBeenCalled();

    handlePointerUp(editor, createPointerEvent({ target }));
    expect(editor.resumeToolbarSync).toHaveBeenCalledTimes(1);
  });

  it('does not suspend toolbar sync when admitting a native text-move drag', () => {
    const editor = createEditor({ selectionHit: true, isSelectionCollapsed: false, selection: rangeSelection });
    const target = createPointerTarget();

    handlePointerDown(editor, createPointerEvent({ target }));

    expect(editor.suspendToolbarSync).not.toHaveBeenCalled();
  });

  it('routes read-only touch pointers to the gesture controller without capturing', () => {
    const editor = createEditor({ readOnly: true });
    const target = createPointerTarget();
    const down = { ...createPointerEvent({ target }), pointerType: 'touch' } as unknown as PointerEvent & { currentTarget: HTMLElement };

    handlePointerDown(editor, down);

    expect(editor.gesture.handlePointerDown).toHaveBeenCalledWith(down, { page: 0, x: 10, y: 20 }, null);
    expect(target.setPointerCapture).not.toHaveBeenCalled();
    expect(editor.enqueue).not.toHaveBeenCalled();
  });

  it('does not toggle a fold on read-only touch down and starts a gesture instead', () => {
    const editor = createEditor({ readOnly: true });
    editor.interactiveHitTest = vi.fn(() => ({ type: 'fold_title', id: 'fold-1' })) as unknown as Editor['interactiveHitTest'];
    const target = createPointerTarget();
    const down = { ...createPointerEvent({ target }), pointerType: 'touch' } as unknown as PointerEvent & { currentTarget: HTMLElement };

    handlePointerDown(editor, down);

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(editor.gesture.handlePointerDown).toHaveBeenCalledWith(down, { page: 0, x: 10, y: 20 }, null);
  });

  it('sends edit-mode touch pointers through the regular pointer path', () => {
    const editor = createEditor();
    const target = createPointerTarget({ captured: true });
    const down = { ...createPointerEvent({ target }), pointerType: 'touch' } as unknown as PointerEvent & { currentTarget: HTMLElement };

    handlePointerDown(editor, down);

    expect(editor.gesture.handlePointerDown).not.toHaveBeenCalled();
    expect(target.setPointerCapture).toHaveBeenCalledWith(1);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'selection', op: { type: 'set_at', page: 0, x: 10, y: 20 } });
  });

  it('allows collapse for regular drag selection extension', () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    const editor = createEditor();
    const target = createPointerTarget({ captured: true });
    editor.clientToLocal = vi.fn((clientX: number, clientY: number) => ({ page: 0, x: clientX - 100, y: clientY - 200 }));

    handlePointerDown(editor, createPointerEvent({ target, clientX: 110, clientY: 220 }));
    editor.enqueue.mockClear();

    handlePointerMove(editor, createPointerEvent({ target, clientX: 130, clientY: 220 }));

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: collapsedSelection.anchor,
        head_page: 0,
        head_x: 30,
        head_y: 20,
        base_selection: undefined,
        allow_collapse: true,
      },
    });
    vi.unstubAllGlobals();
  });
});
