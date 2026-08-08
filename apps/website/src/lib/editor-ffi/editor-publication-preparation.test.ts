import { describe, expect, it, vi } from 'vitest';
import { resolveEditorSurfacePreparation } from './editor-publication.svelte';
import { EditorScrollScope } from './scroll.svelte';
import type { Editor, EditorSnapshot } from './editor.svelte';

describe('editor publication preparation', () => {
  it('plans surfaces and reveal from the anchor-corrected candidate scroll', () => {
    const snapshot = {
      revision: 1,
      pageSizes: Array.from({ length: 4 }, () => ({ width: 600, height: 1000 })),
      trackedRanges: [
        {
          id: 'target',
          rects: [{ page_idx: 0, rect: { x: 0, y: 100, width: 1, height: 20 } }],
        },
      ],
      selection: undefined,
      selectionEndpoints: undefined,
      cursor: undefined,
      rootAttrs: undefined,
    } as EditorSnapshot;
    let scrollTop = 100;
    const editor = {
      destroyed: false,
      appliedSnapshot: snapshot,
      appliedRevision: snapshot.revision,
      publishedRevision: snapshot.revision,
      published: { snapshot, frames: new Map([[0, {}]]) },
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      extensionAreaEl: null,
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => scrollTop,
        getScrollHeight: () => 4000,
        scrollTo: vi.fn((options: ScrollToOptions) => {
          scrollTop = options.top ?? scrollTop;
        }),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
      trackedRangeForSnapshot: (id: string, candidate: EditorSnapshot) => {
        const range = candidate.trackedRanges.find((item) => item.id === id);
        return range ? { ...range, rects: [{ page_idx: 2, rect: { x: 0, y: 100, width: 1, height: 20 } }] } : undefined;
      },
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));
    scroll.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'result_reveal' });
    const preparation = resolveEditorSurfacePreparation(editor, scroll, 2100);

    expect(preparation?.scrollIntent).toEqual({ type: 'scroll_to', y: 2040 });
    expect(preparation?.requiredPages).toEqual(new Set([1, 2]));
  });
});
