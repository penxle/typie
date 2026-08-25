import { afterEach, describe, expect, it, vi } from 'vitest';
import { resolveEditorSurfacePreparation } from './editor-publication.svelte';
import { EditorScrollScope } from './scroll.svelte';
import type { Editor, EditorSnapshot } from './editor.svelte';

const snapshot = (overrides: Partial<EditorSnapshot>): EditorSnapshot => ({
  revision: 1,
  cursor: undefined,
  placeholder: undefined,
  selection: undefined,
  selectionEndpoints: undefined,
  lastHistoryTag: undefined,
  pageSizes: [],
  pageBackingSizes: [],
  externalElements: [],
  tableOverlays: [],
  linkRects: [],
  pageData: new Map(),
  rootAttrs: undefined,
  rootModifiers: [],
  trackedRanges: [],
  ...overrides,
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('editor publication preparation', () => {
  it('completes a current-selection reveal when the candidate has no selection geometry', async () => {
    const position = { node: 'hidden', offset: 0, affinity: 'downstream' as const };
    const candidate = snapshot({
      selection: { anchor: position, head: position },
      pageSizes: [{ width: 600, height: 1000 }],
    });
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: candidate.revision,
      published: { snapshot: candidate, frames: new Map([[0, {}]]) },
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, 0, 600, 1000),
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 1000,
        scrollTo: vi.fn(),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
      clientToLocal: vi.fn(() => null),
      captureSelectionViewportAnchor: vi.fn(() => void 0),
      captureViewportAnchorAt: vi.fn(() => void 0),
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));
    const presentation = scroll.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'reveal' });

    expect(scroll.prepareViewportAnchorPublication(candidate).type).toBe('ready');
    const preparation = resolveEditorSurfacePreparation(editor, scroll);
    expect(preparation?.scrollIntent).toEqual({ type: 'no_scroll' });
    const request = preparation?.pendingRequest;
    expect(request).not.toBeNull();
    if (!request || !preparation?.scrollIntent) throw new Error('Expected a pending no-scroll reveal');
    expect(scroll.applyPending(request, candidate, preparation.scrollIntent)).toBe(true);
    await expect(presentation).resolves.toBeUndefined();
  });

  it('plans surfaces and reveal from the anchor-corrected candidate scroll', () => {
    const candidate = snapshot({
      revision: 1,
      pageSizes: Array.from({ length: 4 }, () => ({ width: 600, height: 1000 })),
      trackedRanges: [
        {
          id: 'target',
          group: 'comment',
          anchor: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          head: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          metadata: '',
          rects: [{ page_idx: 0, rect: { x: 0, y: 100, width: 1, height: 20 } }],
          text: '',
        },
      ],
    });
    let scrollTop = 100;
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: candidate.revision,
      published: { snapshot: candidate, frames: new Map([[0, {}]]) },
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, -100, 600, 4000),
      },
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
    scroll.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });
    const preparation = resolveEditorSurfacePreparation(editor, scroll, 2100);

    expect(preparation?.scrollIntent).toEqual({ type: 'scroll_to', y: 2040 });
    expect(preparation?.requiredPages).toEqual(new Set([1, 2]));
  });

  it('reveals the minimum target height together with a tracked item', () => {
    const candidate = snapshot({
      pageSizes: [{ width: 600, height: 1000 }],
      trackedRanges: [
        {
          id: 'target',
          group: 'comment',
          anchor: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          head: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          metadata: '',
          rects: [{ page_idx: 0, rect: { x: 0, y: 100, width: 1, height: 20 } }],
          text: '',
        },
      ],
    });
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: candidate.revision,
      published: { snapshot: candidate, frames: new Map([[0, {}]]) },
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, 0, 600, 1000),
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 1000,
        scrollTo: vi.fn(),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
      trackedRangeForSnapshot: (id: string, current: EditorSnapshot) => current.trackedRanges.find((range) => range.id === id),
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));
    scroll.scrollIntoView({
      target: { type: 'tracked_item', id: 'target', minimumHeight: 500 },
      policy: 'reveal',
    });

    const preparation = resolveEditorSurfacePreparation(editor, scroll);

    expect(preparation?.scrollIntent).toEqual({ type: 'scroll_to', y: 40 });
  });

  it('prepares destination surfaces when reduced motion turns a smooth reveal into an instant reveal', () => {
    const candidate = snapshot({
      pageSizes: Array.from({ length: 3 }, () => ({ width: 600, height: 1000 })),
      trackedRanges: [
        {
          id: 'target',
          group: 'comment',
          anchor: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          head: { node: 'paragraph', offset: 0, affinity: 'downstream' },
          metadata: '',
          rects: [{ page_idx: 2, rect: { x: 0, y: 100, width: 1, height: 20 } }],
          text: '',
        },
      ],
    });
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: candidate.revision,
      published: { snapshot: candidate, frames: new Map([[0, {}]]) },
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, 0, 600, 3040),
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => 0,
        getScrollHeight: () => 3040,
        scrollTo: vi.fn(),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
      trackedRangeForSnapshot: (id: string, snapshot: EditorSnapshot) => snapshot.trackedRanges.find((range) => range.id === id),
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));
    vi.stubGlobal('matchMedia', () => ({ matches: false }));
    scroll.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });

    const animated = resolveEditorSurfacePreparation(editor, scroll);

    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    scroll.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const reducedMotion = resolveEditorSurfacePreparation(editor, scroll);

    expect(animated?.requiredPages.has(2)).toBe(false);
    expect(reducedMotion?.requiredPages.has(2)).toBe(true);
  });

  it('clamps surface planning to a candidate document that became shorter than the current scroll', () => {
    const candidate = snapshot({
      revision: 2,
      pageSizes: [{ width: 600, height: 1000 }],
    });
    const scrollTop = 90_000;
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: 1,
      published: undefined,
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set([9]),
      scrollRootEl: {} as HTMLElement,
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, -scrollTop, 600, 1000),
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => scrollTop,
        getScrollHeight: () => 100_000,
        scrollTo: vi.fn(),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));

    const preparation = resolveEditorSurfacePreparation(editor, scroll);

    expect(preparation?.requiredPages).toEqual(new Set([0]));
  });

  it('keeps planning in the enclosing window scroll range beyond the editor document', () => {
    const candidate = snapshot({
      revision: 1,
      pageSizes: [{ width: 600, height: 1000 }],
    });
    const scrollTop = 3000;
    const editor = {
      destroyed: false,
      appliedSnapshot: candidate,
      appliedRevision: candidate.revision,
      publishedRevision: candidate.revision,
      published: undefined,
      viewport: { height: 400 },
      scaleFactor: 1,
      displayZoom: 1,
      activeSurfacePages: new Set<number>(),
      scrollRootEl: null,
      extensionAreaEl: {
        getBoundingClientRect: () => new DOMRect(0, -scrollTop, 600, 1000),
      },
      scrollViewport: {
        getRect: () => new DOMRect(0, 0, 600, 400),
        getScrollTop: () => scrollTop,
        getScrollHeight: () => 4000,
        scrollTo: vi.fn(),
      },
      requestPublication: vi.fn(),
      safeDisplayZoom: () => 1,
    } as unknown as Editor;
    const scroll = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));

    const preparation = resolveEditorSurfacePreparation(editor, scroll);

    expect(preparation?.requiredPages).toEqual(new Set());
  });
});
