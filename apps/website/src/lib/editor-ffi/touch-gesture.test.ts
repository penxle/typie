import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { LONG_PRESS_MS } from './constants';
import { computeSelectionHandleVisual, computeTouchContextMenuPosition, TouchGestureController } from './gesture.svelte';
import type { SelectionEndpoints } from '@typie/editor-ffi/browser';

const pageRect = (left: number, top: number, width: number, height: number): DOMRect =>
  ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) }) as DOMRect;

const viewport = (left: number, top: number, width: number, height: number) => ({ left, top, width, height });

const endpoints = (
  from: { page_idx: number; x: number; y: number; width: number; height: number },
  to: { page_idx: number; x: number; y: number; width: number; height: number },
): SelectionEndpoints => ({
  from: { page_idx: from.page_idx, rect: { x: from.x, y: from.y, width: from.width, height: from.height } },
  to: { page_idx: to.page_idx, rect: { x: to.x, y: to.y, width: to.width, height: to.height } },
  from_position: { node: 'text', offset: 0, affinity: 'downstream' },
  to_position: { node: 'text', offset: 1, affinity: 'downstream' },
});

describe('computeTouchContextMenuPosition', () => {
  it('places menu above selection when there is space above', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: 100, y: 200, width: 50, height: 20 }, { page_idx: 0, x: 200, y: 200, width: 30, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result).not.toBeNull();
    expect(result?.placement).toBe('top');
    expect(result?.x).toBe(165);
    expect(result?.y).toBe(200);
  });

  it('places menu below when space above is insufficient', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: 100, y: 10, width: 50, height: 20 }, { page_idx: 0, x: 200, y: 10, width: 30, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result?.placement).toBe('bottom');
    expect(result?.y).toBe(30);
  });

  it('returns null when no pageRect matches the selection page_idx', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 5, x: 0, y: 0, width: 10, height: 10 }, { page_idx: 5, x: 10, y: 0, width: 10, height: 10 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result).toBeNull();
  });

  it('clamps x to viewport with padding when selection is at the left edge', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: -10, y: 100, width: 5, height: 20 }, { page_idx: 0, x: -10, y: 100, width: 5, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result?.x).toBe(8);
  });

  it('clamps x to viewport with padding when selection is at the right edge', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints(
        { page_idx: 0, x: 900, y: 100, width: 100, height: 20 },
        { page_idx: 0, x: 900, y: 100, width: 100, height: 20 },
      ),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result?.x).toBe(792);
  });

  it('clamps y to viewport with padding', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: 100, y: 990, width: 50, height: 20 }, { page_idx: 0, x: 200, y: 990, width: 30, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result?.placement).toBe('top');
    expect(result?.y).toBeLessThanOrEqual(992);
  });

  it('respects zoom when computing client-space rects', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: 100, y: 200, width: 50, height: 20 }, { page_idx: 0, x: 100, y: 200, width: 50, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000)],
      zoom: 2,
      viewport: viewport(0, 0, 800, 1000),
    });
    expect(result?.placement).toBe('top');
    expect(result?.x).toBe(250);
    expect(result?.y).toBe(400);
  });

  it('spans page boundaries when from and to are on different pages', () => {
    const result = computeTouchContextMenuPosition({
      endpoints: endpoints({ page_idx: 0, x: 100, y: 990, width: 50, height: 20 }, { page_idx: 1, x: 100, y: 10, width: 50, height: 20 }),
      pageRects: [pageRect(0, 0, 800, 1000), pageRect(0, 1024, 800, 1000)],
      zoom: 1,
      viewport: viewport(0, 0, 800, 2200),
    });
    expect(result).not.toBeNull();
    expect(result?.x).toBe(125);
  });
});

const makePointerEvent = ({
  pointerId = 1,
  clientX = 110,
  clientY = 220,
  isPrimary = true,
}: { pointerId?: number; clientX?: number; clientY?: number; isPrimary?: boolean } = {}): PointerEvent =>
  ({
    pointerId,
    clientX,
    clientY,
    isPrimary,
    preventDefault: vi.fn(),
  }) as unknown as PointerEvent;

const createGestureEditor = () => {
  const selection = endpoints(
    { page_idx: 0, x: 100, y: 200, width: 50, height: 20 },
    { page_idx: 0, x: 200, y: 200, width: 30, height: 20 },
  );
  const menuItems = [{ label: '링크 열기', onclick: vi.fn() }];

  return {
    selection,
    readOnly: true,
    pageSizes: [{ width: 800, height: 1000 }],
    pageEls: {
      0: {
        getBoundingClientRect: () => pageRect(0, 0, 800, 1000),
      },
    },
    safeDisplayZoom: vi.fn(() => 1),
    selectionEndpoints: vi.fn(() => selection),
    selectionHitTest: vi.fn(() => true),
    interactiveHitTest: vi.fn(() => ({ type: 'text' })),
    collectContextMenuContributions: vi.fn(() => menuItems),
    openContextMenu: vi.fn(),
    closeContextMenu: vi.fn(),
    clientToLocal: vi.fn(() => ({ page: 0, x: 120, y: 220 })),
    enqueue: vi.fn(),
    flush: vi.fn(),
  };
};

describe('TouchGestureController', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('includes context menu contributions when long-press opens a touch menu on a selection', () => {
    const editor = createGestureEditor();
    const controller = new TouchGestureController(editor as never);

    controller.handlePointerDown(makePointerEvent(), { page: 0, x: 120, y: 220 });
    vi.advanceTimersByTime(LONG_PRESS_MS);

    expect(editor.collectContextMenuContributions).toHaveBeenCalledWith({
      hit: { type: 'text' },
      clientX: 110,
      clientY: 220,
    });
    expect(editor.openContextMenu).toHaveBeenCalledWith({
      x: 165,
      y: 200,
      source: 'touch',
      placement: 'top',
      extraItems: [{ label: '링크 열기', onclick: expect.any(Function) }],
    });
  });

  it('extends the selection from a dragged handle and reopens the touch menu on pointer up', () => {
    const editor = createGestureEditor();
    editor.clientToLocal = vi.fn(() => ({ page: 0, x: 250, y: 240 }));
    const controller = new TouchGestureController(editor as never);

    controller.handleSelectionHandlePointerDown('from', makePointerEvent({ clientX: 110, clientY: 220 }));
    controller.handleSelectionHandlePointerMove(makePointerEvent({ clientX: 250, clientY: 240 }));
    controller.handleSelectionHandlePointerUp(makePointerEvent({ clientX: 250, clientY: 240 }));

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'selection',
      op: {
        type: 'extend_to',
        anchor: editor.selection.to_position,
        head_page: 0,
        head_x: 250,
        head_y: 240,
        base_selection: undefined,
        allow_collapse: false,
      },
    });
    expect(editor.flush).toHaveBeenCalled();
    expect(editor.openContextMenu).toHaveBeenLastCalledWith({
      x: 165,
      y: 200,
      source: 'touch',
      placement: 'top',
      extraItems: [{ label: '링크 열기', onclick: expect.any(Function) }],
    });
  });
});

describe('computeSelectionHandleVisual', () => {
  it('places the end (to) handle at the anchor rect', () => {
    const result = computeSelectionHandleVisual({
      kind: 'to',
      anchorRect: pageRect(200, 200, 30, 20),
    });
    expect(result).toEqual({ left: 179, top: 196, touchHeight: 44, paintLeft: 14, paintTop: 4, stemHeight: 20 });
  });

  it('places the start (from) handle above the line with its stem pointing down', () => {
    const result = computeSelectionHandleVisual({
      kind: 'from',
      anchorRect: pageRect(100, 200, 50, 20),
    });
    expect(result).toEqual({ left: 77, top: 180, touchHeight: 44, paintLeft: 14, paintTop: 4, stemHeight: 20 });
  });

  it('uses the viewport anchor rect without surface conversion', () => {
    const result = computeSelectionHandleVisual({
      kind: 'to',
      anchorRect: pageRect(440, 495, 60, 40),
    });
    expect(result).toEqual({ left: 419, top: 495, touchHeight: 56, paintLeft: 14, paintTop: 0, stemHeight: 40 });
  });
});
