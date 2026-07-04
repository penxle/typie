import { describe, expect, it } from 'vitest';
import { boundingClientRect, pageRectsToVirtualElement } from './geometry';
import type { Editor } from './editor.svelte';

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

describe('pageRectsToVirtualElement', () => {
  it('keeps an empty bounding rect fallback for empty virtual elements', () => {
    const editor = { pageEls: [], safeDisplayZoom: () => 1 } as unknown as Editor;
    const virtualElement = pageRectsToVirtualElement(editor, []);

    const rect = virtualElement.getBoundingClientRect();
    expect(rect.width).toBe(0);
    expect(rect.height).toBe(0);
    expect([...(virtualElement.getClientRects?.() ?? [])]).toEqual([]);
  });
});
