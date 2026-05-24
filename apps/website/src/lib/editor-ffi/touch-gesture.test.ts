import { describe, expect, it } from 'vitest';
import { computeTouchContextMenuPosition } from './gesture.svelte';
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
