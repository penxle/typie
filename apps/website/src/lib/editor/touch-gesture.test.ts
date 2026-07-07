import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TouchGestureController } from './touch-gesture.svelte';
import type { Selection } from './types';

const makePointerEvent = ({
  pointerId = 1,
  clientX = 110,
  clientY = 220,
  isPrimary = true,
}: { pointerId?: number; clientX?: number; clientY?: number; isPrimary?: boolean } = {}): PointerEvent =>
  ({
    pointerId,
    pointerType: 'touch',
    clientX,
    clientY,
    isPrimary,
    preventDefault: vi.fn(),
  }) as unknown as PointerEvent;

const pageRect = (left: number, top: number, width: number, height: number): DOMRect =>
  ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) }) as DOMRect;

const selection = {
  collapsed: false,
  cmp: 1,
  selectedBlockCount: 0,
  anchor: { nodeId: 'text', offset: 0, affinity: 'downstream' },
  head: { nodeId: 'text', offset: 1, affinity: 'downstream' },
  anchorBounds: { pageIdx: 0, bounds: { x: 100, y: 200, width: 50, height: 20 } },
  headBounds: { pageIdx: 0, bounds: { x: 200, y: 200, width: 30, height: 20 } },
  precedingText: '',
  followingText: '',
} as unknown as Selection;

const createGestureEditor = ({ selectionHit = false } = {}) => ({
  readOnly: true,
  selection,
  contextMenu: { isOpen: false, source: 'touch' },
  layout: { layoutMode: { type: 'continuous' } },
  displayZoom: 1,
  pageContainerEls: [
    {
      getBoundingClientRect: () => pageRect(0, 0, 800, 1000),
    },
  ],
  scrollViewport: null,
  isSelectionHit: vi.fn(() => selectionHit),
  resolvePointerCoordinateFromClient: vi.fn(() => ({ pageIdx: 0, x: 120, y: 220 })),
  dispatch: vi.fn(),
  openContextMenu: vi.fn(),
  closeContextMenu: vi.fn(),
  runAfterSettled: vi.fn((task: () => void) => task()),
});

describe('TouchGestureController', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('routes a handle tap that never drags through the regular tap collapse path', () => {
    const editor = createGestureEditor();
    const controller = new TouchGestureController(editor as never);

    controller.handleSelectionHandlePointerDown('from', makePointerEvent({ clientX: 110, clientY: 220 }));
    controller.handleSelectionHandlePointerUp(makePointerEvent({ clientX: 110, clientY: 220 }));

    expect(editor.dispatch).toHaveBeenNthCalledWith(1, {
      type: 'pointerDown',
      pageIdx: 0,
      x: 120,
      y: 220,
      clickCount: 1,
      button: 'primary',
      modifier: { shift: false, ctrl: false, alt: false, meta: false },
    });
    expect(editor.dispatch).toHaveBeenNthCalledWith(2, {
      type: 'pointerUp',
      pageIdx: 0,
      x: 120,
      y: 220,
      button: 'primary',
      modifier: { shift: false, ctrl: false, alt: false, meta: false },
    });
    expect(editor.dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'extendSelectionTo' }));
    expect(editor.openContextMenu).not.toHaveBeenCalled();
  });

  it('extends the selection from a dragged handle and reopens the touch menu on pointer up', () => {
    const editor = createGestureEditor();
    const controller = new TouchGestureController(editor as never);

    controller.handleSelectionHandlePointerDown('from', makePointerEvent({ clientX: 110, clientY: 220 }));
    controller.handleSelectionHandlePointerMove(makePointerEvent({ clientX: 130, clientY: 220 }));
    controller.handleSelectionHandlePointerUp(makePointerEvent({ clientX: 130, clientY: 220 }));

    expect(editor.dispatch).toHaveBeenCalledWith({
      type: 'extendSelectionTo',
      anchorPageIdx: 0,
      anchorX: 200,
      anchorY: 210,
      headPageIdx: 0,
      headX: 120,
      headY: 220,
      doubleTapInitialRange: undefined,
    });
    expect(editor.openContextMenu).toHaveBeenCalledWith(expect.objectContaining({ source: 'touch' }));
  });
});
