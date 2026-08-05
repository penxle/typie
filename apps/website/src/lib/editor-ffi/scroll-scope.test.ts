import { describe, expect, it, vi } from 'vitest';
import { EditorRequest, EditorUpdate } from './editor-update';
import { EditorScrollScope } from './scroll.svelte';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorSnapshot } from './editor.svelte';

const trackedSnapshot = (id: string, rect: PageRect): EditorSnapshot =>
  ({
    revision: 1,
    selection: undefined,
    selectionEndpoints: undefined,
    cursor: undefined,
    trackedRanges: [{ id, rects: [rect] }],
    rootAttrs: undefined,
  }) as EditorSnapshot;

describe('EditorRequest', () => {
  it('replaces an existing before-publish registration and runs only the latest one', () => {
    const request = new EditorRequest();
    const first = vi.fn();
    const discardFirst = vi.fn();
    const latest = vi.fn();
    const discardLatest = vi.fn();
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 0, width: 1, height: 20 },
    });

    request.beforePublish(first, discardFirst);
    request.beforePublish(latest, discardLatest);
    request.runBeforePublish(new EditorUpdate(7, snapshot, [], [], async () => ({ type: 'published', revision: 7 })));

    expect(discardFirst).toHaveBeenCalledOnce();
    expect(first).not.toHaveBeenCalled();
    expect(latest).toHaveBeenCalledOnce();
    expect(discardLatest).not.toHaveBeenCalled();
  });

  it('discards the latest before-publish registration after replacing an earlier one', () => {
    const request = new EditorRequest();
    const discardFirst = vi.fn();
    const discardLatest = vi.fn();

    request.beforePublish(vi.fn(), discardFirst);
    request.beforePublish(vi.fn(), discardLatest);
    request.discard();

    expect(discardFirst).toHaveBeenCalledOnce();
    expect(discardLatest).toHaveBeenCalledOnce();
  });
});

function setup(snapshot: EditorSnapshot) {
  const scrollTo = vi.fn();
  const requestPublication = vi.fn();
  const editor = {
    destroyed: false,
    appliedSnapshot: snapshot,
    published: {
      snapshot,
      frames: new Map([
        [0, {}],
        [1, {}],
      ]),
    },
    viewport: { height: 400 },
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
    requestPublication,
  } as unknown as Editor;
  return {
    editor,
    requestPublication,
    scrollTo,
    scope: new EditorScrollScope(editor, () => ({ enabled: false, position: undefined })),
  };
}

describe('EditorScrollScope', () => {
  it('keeps a declared reveal ineligible until the installing revision binds it', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const request = scope.declare({ target: { type: 'tracked_item', id: 'target' }, mode: 'nearest' });

    expect(scope.activateForRevision(7)).toBeNull();
    expect(scope.bind(request, 7)).toBe(true);
    expect(scope.activateForRevision(6)).toBeNull();
    expect(scope.activateForRevision(7)).toBe(request);
  });

  it('binds scrollIntoView called during update admission to that synchronous update', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const admission = new EditorRequest();

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'nearest' }, admission);
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');
    expect(request.targetRevision).toBeNull();

    admission.runBeforePublish(new EditorUpdate(7, snapshot, [], [], async () => ({ type: 'published', revision: 7 })));
    expect(scope.activateForRevision(7)).toBe(request);
  });

  it('returns the request-bound presentation completion to the caller', async () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);

    const presentation = scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, mode: 'nearest' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');

    expect(presentation).toBe(request.presentation);

    let presented = false;
    void presentation?.then(() => {
      presented = true;
    });
    await Promise.resolve();
    expect(presented).toBe(false);

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    await expect(presentation).resolves.toBeUndefined();
  });

  it('re-resolves current-selection and tracked-item targets from newer eligible revisions', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const tracked = scope.declare({ target: { type: 'tracked_item', id: 'target' }, mode: 'nearest' });
    expect(scope.bind(tracked, 7)).toBe(true);
    expect(scope.activateForRevision(8)).toBe(tracked);

    const selection = scope.declare({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    expect(scope.bind(selection, 7)).toBe(true);
    expect(scope.activateForRevision(8)).toBe(selection);
  });

  it('completes a superseded reveal without applying it', async () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const old = scope.declare({ target: { type: 'tracked_item', id: 'old' }, mode: 'nearest' });
    const oldPresentation = old.presentation;

    scope.declare({ target: { type: 'tracked_item', id: 'new' }, mode: 'nearest' });

    await expect(oldPresentation).resolves.toBeUndefined();
  });

  it('cancels a pending automatic reveal when direct manipulation takes control', async () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { requestPublication, scope } = setup(snapshot);
    const request = scope.declare({ target: { type: 'tracked_item', id: 'target' }, mode: 'nearest' });
    const presentation = request.presentation;
    requestPublication.mockClear();

    scope.cancel();

    await expect(presentation).resolves.toBeUndefined();
    expect(scope.pendingRequest).toBeNull();
    expect(requestPublication).toHaveBeenCalledOnce();
  });

  it('keeps the existing latest-request-wins reveal contract', () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { requestPublication, scope } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'old' }, mode: 'nearest' });
    const old = scope.pendingRequest;
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'new' }, mode: 'nearest' });

    expect(scope.pendingRequest?.target).toEqual({ type: 'tracked_item', id: 'new' });
    expect(requestPublication).toHaveBeenCalledTimes(2);
    expect(old).not.toBe(scope.pendingRequest);
  });

  it('applies only the request accepted with the matching publication', () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { requestPublication, scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'old' }, mode: 'nearest' });
    const old = scope.pendingRequest;
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'new' }, mode: 'nearest' });
    const current = scope.pendingRequest;
    if (!old || !current) throw new Error('Expected both scroll requests');
    requestPublication.mockClear();

    expect(scope.applyPending(old, snapshot, { type: 'scroll_to', y: 100 })).toBe(false);
    expect(scope.applyPending(current, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 580, behavior: 'smooth' });
    expect(scope.pendingRequest).toBeNull();
    expect(requestPublication).toHaveBeenCalledOnce();
  });

  it('applies a preverified instant destination without mounted page geometry', () => {
    const head: PageRect = {
      page_idx: 1,
      rect: { x: 0, y: 600, width: 1, height: 20 },
    };
    const position = { node: 'text', offset: 0, affinity: 'downstream' as const };
    const snapshot = {
      ...trackedSnapshot('unused', head),
      cursor: { page_idx: head.page_idx, line: head.rect },
      selection: { anchor: position, head: position },
      selectionEndpoints: {
        from: head,
        to: head,
        from_position: position,
        to_position: position,
      },
    } as EditorSnapshot;
    const { editor, scope, scrollTo } = setup(snapshot);
    editor.pageEls[1] = undefined;

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 777 })).toBe(true);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 777, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
  });

  it('keeps an unresolved instant reveal pending but completes an explicit no-scroll result', async () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const { scope, scrollTo } = setup(snapshot);
    const presentation = scope.scrollIntoView({
      target: { type: 'tracked_item', id: 'target' },
      mode: 'nearest',
      behavior: 'instant',
    });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');

    expect(scope.applyPending(request, snapshot, { type: 'unresolved' })).toBe(false);
    expect(scope.pendingRequest).toBe(request);
    expect(scrollTo).not.toHaveBeenCalled();

    expect(scope.applyPending(request, snapshot, { type: 'no_scroll' })).toBe(true);
    await expect(presentation).resolves.toBeUndefined();
    expect(scope.pendingRequest).toBeNull();
    expect(scrollTo).not.toHaveBeenCalled();
  });

  it('applies the precomputed destination for the accepted snapshot', () => {
    const head: PageRect = {
      page_idx: 0,
      rect: { x: 0, y: 600, width: 1, height: 20 },
    };
    const position = { node: 'text', offset: 0, affinity: 'downstream' as const };
    const snapshot = {
      ...trackedSnapshot('unused', head),
      cursor: { page_idx: head.page_idx, line: head.rect },
      selection: { anchor: position, head: position },
      selectionEndpoints: {
        from: head,
        to: head,
        from_position: position,
        to_position: position,
      },
    } as EditorSnapshot;
    const { scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, mode: 'nearest' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 280 })).toBe(true);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 280, behavior: 'instant' });
  });

  it('starts a smooth reveal without requiring a frame for the target page', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 1,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const { editor, scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, mode: 'nearest' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');

    editor.published = {
      snapshot,
      frames: new Map([[0, { revision: snapshot.revision, surfaceKey: 1, frameKey: 1, canvas: document.createElement('canvas') }]]),
    };
    editor.pageEls[1] = undefined;

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 900 })).toBe(true);
    expect(scope.pendingRequest).toBeNull();
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 900, behavior: 'smooth' });
  });

  it('completes an eligible reveal as a no-op when the matching snapshot has no target', async () => {
    const snapshot = trackedSnapshot('other', {
      page_idx: 0,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const { scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'missing' }, mode: 'nearest' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');
    const presentation = request.presentation;

    expect(scope.applyPending(request, snapshot, { type: 'no_scroll' })).toBe(true);
    await expect(presentation).resolves.toBeUndefined();
    expect(scope.pendingRequest).toBeNull();
    expect(scrollTo).not.toHaveBeenCalled();
  });
});
