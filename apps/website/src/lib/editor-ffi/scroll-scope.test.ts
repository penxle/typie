import { tick } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import { EditorScrollScope } from './scroll.svelte';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

describe('EditorScrollScope', () => {
  it('resolves the current selection after a synchronous editor update publishes', async () => {
    let selectionHead: PageRect = {
      page_idx: 0,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    };
    const scrollTo = vi.fn();
    const pageEl = {
      getBoundingClientRect: () => new DOMRect(0, 0, 600, 1200),
    } as HTMLDivElement;
    const editor = {
      destroyed: false,
      hasQueuedTick: false,
      appliedRevision: 1,
      published: {
        snapshot: {
          revision: 1,
          selection: {
            anchor: { node: 'text', offset: 0, affinity: 'downstream' as const },
            head: { node: 'text', offset: 0, affinity: 'downstream' as const },
          },
          selectionEndpoints: {
            from: selectionHead,
            to: selectionHead,
            from_position: { node: 'text', offset: 0, affinity: 'downstream' as const },
            to_position: { node: 'text', offset: 0, affinity: 'downstream' as const },
          },
          cursor: undefined,
          trackedRanges: [],
        },
      },
      viewport: { height: 400 },
      rootAttrs: undefined,
      pageEls: { 0: pageEl },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 1200,
        scrollTo,
      },
      safeDisplayZoom: () => 1,
      awaitPublishedRevision: vi.fn(async () => ({ type: 'published' as const, revision: 1 })),
    } as unknown as Editor;
    const scope = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    selectionHead = {
      page_idx: 0,
      rect: { x: 0, y: 600, width: 1, height: 20 },
    };
    const currentSnapshot = editor.published?.snapshot;
    if (!currentSnapshot) throw new Error('Expected an initial publication');
    editor.published = {
      snapshot: {
        ...currentSnapshot,
        selectionEndpoints: {
          ...currentSnapshot.selectionEndpoints,
          from: selectionHead,
          to: selectionHead,
        },
      },
    } as never;

    await tick();

    await vi.waitFor(() => {
      expect(scrollTo).toHaveBeenCalledWith({ top: 280, behavior: 'instant' });
    });
  });

  it('waits for the requested revision before resolving published selection geometry', async () => {
    const { promise: publication, resolve: resolvePublished } = Promise.withResolvers<{ type: 'published'; revision: number }>();
    const staleHead: PageRect = {
      page_idx: 0,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    };
    const publishedHead: PageRect = {
      page_idx: 0,
      rect: { x: 0, y: 600, width: 1, height: 20 },
    };
    const selection = {
      anchor: { node: 'text', offset: 0, affinity: 'downstream' as const },
      head: { node: 'text', offset: 0, affinity: 'downstream' as const },
    };
    const snapshot = (revision: number, head: PageRect) => ({
      revision,
      selection,
      selectionEndpoints: {
        from: head,
        to: head,
        from_position: selection.anchor,
        to_position: selection.head,
      },
      cursor: undefined,
      trackedRanges: [],
    });
    const scrollTo = vi.fn();
    const editor = {
      destroyed: false,
      hasQueuedTick: false,
      appliedRevision: 2,
      published: { snapshot: snapshot(1, staleHead) },
      viewport: { height: 400 },
      rootAttrs: undefined,
      pageEls: {
        0: {
          getBoundingClientRect: () => new DOMRect(0, 0, 600, 1200),
        },
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 1200,
        scrollTo,
      },
      safeDisplayZoom: () => 1,
      awaitPublishedRevision: vi.fn(() => publication),
    } as unknown as Editor;
    const scope = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    await tick();

    expect(scrollTo).not.toHaveBeenCalled();

    editor.published = { snapshot: snapshot(2, publishedHead) } as never;
    resolvePublished({ type: 'published', revision: 2 });

    await vi.waitFor(() => {
      expect(scrollTo).toHaveBeenCalledWith({ top: 280, behavior: 'instant' });
    });
    expect(editor.awaitPublishedRevision).toHaveBeenCalledWith(2);
  });

  it('does not apply an older request after a newer request arrives during layout synchronization', async () => {
    const { promise: publication, resolve: resolvePublished } = Promise.withResolvers<{ type: 'published'; revision: number }>();
    const selection = {
      anchor: { node: 'text', offset: 0, affinity: 'downstream' as const },
      head: { node: 'text', offset: 0, affinity: 'downstream' as const },
    };
    const snapshot = {
      revision: 1,
      selection,
      selectionEndpoints: undefined,
      cursor: undefined,
      trackedRanges: [
        {
          id: 'old',
          rects: [{ page_idx: 0, rect: { x: 0, y: 600, width: 1, height: 20 } }],
        },
        {
          id: 'new',
          rects: [{ page_idx: 0, rect: { x: 0, y: 900, width: 1, height: 20 } }],
        },
      ],
    };
    const scrollTo = vi.fn();
    const editor = {
      destroyed: false,
      hasQueuedTick: false,
      appliedRevision: 1,
      published: { snapshot },
      viewport: { height: 400 },
      rootAttrs: undefined,
      pageEls: {
        0: {
          getBoundingClientRect: () => new DOMRect(0, 0, 600, 1200),
        },
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 1200,
        scrollTo,
      },
      safeDisplayZoom: () => 1,
      awaitPublishedRevision: vi.fn(() => publication),
    } as unknown as Editor;
    let superseded = false;
    const scope = new EditorScrollScope(editor, () => {
      if (!superseded) {
        superseded = true;
        scope.scrollIntoView({ target: { type: 'tracked_item', id: 'new' }, mode: 'nearest' });
      }
      return { enabled: true, position: undefined };
    });

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'old' }, mode: 'typewriter' });
    await tick();
    resolvePublished({ type: 'published', revision: 1 });

    await vi.waitFor(() => {
      expect(scrollTo).toHaveBeenCalledWith({ top: 580, behavior: 'smooth' });
    });
    expect(scrollTo).toHaveBeenCalledTimes(1);
  });
});
