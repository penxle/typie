import { describe, expect, it } from 'vitest';
import {
  boundingClientRect,
  isSelectionCollapsed,
  pageRectsToRevealTargetSpan,
  pageRectsToVirtualElement,
  resolvePageSpans,
  selectionHeadRect,
} from './geometry';
import type { Position } from '@typie/editor-ffi/browser';
import type { Editor, EditorSnapshot } from './editor.svelte';

describe('boundingClientRect', () => {
  it('returns the bounding union of all client rects', () => {
    const rect = boundingClientRect([new DOMRect(20, 40, 10, 20), new DOMRect(5, 80, 20, 10), new DOMRect(40, 30, 5, 5)]);

    expect(rect?.left).toBe(5);
    expect(rect?.top).toBe(30);
    expect(rect?.width).toBe(40);
    expect(rect?.height).toBe(60);
  });

  it('returns null when no rects are present', () => {
    expect(boundingClientRect([])).toBeNull();
  });

  it('ignores non-finite rects', () => {
    expect(boundingClientRect([new DOMRect(NaN, 0, 10, 10)])).toBeNull();
  });
});

describe('resolvePageSpans', () => {
  it('accumulates the same device-scale-rounded height used by each page slot', () => {
    expect(
      resolvePageSpans([{ height: 221 }, { height: 221 }, { height: 221 }], {
        origin: 10,
        displayZoom: 0.75,
        scaleFactor: 2,
        pageGap: 9,
      }),
    ).toEqual([
      { page: 0, top: 10, bottom: 176 },
      { page: 1, top: 185, bottom: 351 },
      { page: 2, top: 360, bottom: 526 },
    ]);
  });
});

describe('pageRectsToRevealTargetSpan', () => {
  it('combines valid page-local rects into one zoomed document span', () => {
    const pages = [
      { page: 0, top: 100, bottom: 300 },
      { page: 1, top: 320, bottom: 520 },
    ];

    expect(
      pageRectsToRevealTargetSpan(
        [
          { page_idx: 0, rect: { x: 0, y: 20, width: 10, height: 10 } },
          { page_idx: 1, rect: { x: 0, y: 40, width: 10, height: 20 } },
          { page_idx: 2, rect: { x: 0, y: 0, width: 10, height: 10 } },
        ],
        pages,
        2,
      ),
    ).toEqual({ targetTop: 140, targetBottom: 440 });
  });
});

describe('pageRectsToVirtualElement', () => {
  it('keeps an empty bounding rect fallback for empty virtual elements', () => {
    const editor = { pageEls: [], safeDisplayZoom: () => 1 } as unknown as Editor;
    const virtualElement = pageRectsToVirtualElement(editor, []);

    const rect = virtualElement.getBoundingClientRect();
    expect(rect.width).toBe(0);
    expect(rect.height).toBe(0);
    expect([...(virtualElement.getClientRects?.() ?? [])]).toEqual([]);
  });

  it('uses accepted page layout geometry without requiring an active surface frame', () => {
    const editor = {
      published: { frames: new Map() },
      pageEls: {
        2: { getBoundingClientRect: () => new DOMRect(40, 2000, 600, 800) },
      },
      safeDisplayZoom: () => 1,
    } as unknown as Editor;
    const virtualElement = pageRectsToVirtualElement(editor, [{ page_idx: 2, rect: { x: 20, y: 100, width: 40, height: 30 } }]);

    const rect = virtualElement.getBoundingClientRect();
    expect(rect.left).toBe(60);
    expect(rect.top).toBe(2100);
    expect(rect.width).toBe(40);
    expect(rect.height).toBe(30);
  });
});

describe('isSelectionCollapsed', () => {
  const anchor = { node: 'text', offset: 1, affinity: 'downstream' as const };

  it('treats an absent selection and equal endpoints as collapsed', () => {
    expect(isSelectionCollapsed(undefined)).toBe(true);
    expect(isSelectionCollapsed({ anchor, head: anchor })).toBe(true);
  });

  it('compares node, offset, and affinity', () => {
    expect(isSelectionCollapsed({ anchor, head: { ...anchor, affinity: 'upstream' } })).toBe(false);
  });
});

describe('selectionHeadRect', () => {
  const from = { page_idx: 0, rect: { x: 10, y: 20, width: 1, height: 10 } };
  const to = { page_idx: 1, rect: { x: 30, y: 40, width: 1, height: 10 } };
  const anchor = { node: 'text', offset: 1, affinity: 'downstream' as const };
  const head = { node: 'text', offset: 4, affinity: 'upstream' as const };

  const snapshot = (toPosition: Position = head) =>
    ({
      selection: { anchor, head },
      selectionEndpoints: {
        from,
        to,
        from_position: anchor,
        to_position: toPosition,
      },
    }) as EditorSnapshot;

  it('selects the endpoint matching the selection head', () => {
    expect(selectionHeadRect(snapshot())).toEqual(to);
    expect(selectionHeadRect(snapshot(anchor))).toEqual(from);
  });

  it('falls back to the cursor line when selection geometry is unavailable', () => {
    const cursorLine = { x: 50, y: 60, width: 1, height: 12 };
    const cursorSnapshot = {
      selection: undefined,
      selectionEndpoints: undefined,
      cursor: { page_idx: 2, line: cursorLine },
    } as EditorSnapshot;

    expect(selectionHeadRect(cursorSnapshot)).toEqual({ page_idx: 2, rect: cursorLine });
    expect(selectionHeadRect(undefined)).toBeNull();
  });

  it('uses the cursor line for a collapsed selection', () => {
    const cursorLine = { x: 50, y: 60, width: 1, height: 12 };
    const cursorSnapshot = {
      ...snapshot(),
      selection: { anchor, head: anchor },
      cursor: { page_idx: 2, line: cursorLine },
    } as EditorSnapshot;

    expect(selectionHeadRect(cursorSnapshot)).toEqual({ page_idx: 2, rect: cursorLine });
  });
});
