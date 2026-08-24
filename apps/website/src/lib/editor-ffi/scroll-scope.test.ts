import { afterEach, describe, expect, it, vi } from 'vitest';
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
    pageSizes: [
      { width: 600, height: 1200 },
      { width: 600, height: 1200 },
    ],
    rootAttrs: undefined,
  }) as EditorSnapshot;

const selectionSnapshot = (collapsed: boolean, rect: PageRect): EditorSnapshot => {
  const anchor = { node: 'text', offset: 0, affinity: 'downstream' as const };
  const head = collapsed ? anchor : { ...anchor, offset: 1 };
  return {
    ...trackedSnapshot('unused', rect),
    cursor: collapsed ? { page_idx: rect.page_idx, line: rect.rect } : undefined,
    selection: { anchor, head },
    selectionEndpoints: {
      from: rect,
      to: rect,
      from_position: anchor,
      to_position: head,
    },
  } as EditorSnapshot;
};

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

function setup(snapshot: EditorSnapshot, typewriter?: { enabled: boolean; position: number | undefined }) {
  const typewriterPreferences = typewriter ?? { enabled: false, position: undefined };
  let animationTime = 0;
  let nextAnimationFrameId = 1;
  const animationFrames = new Map<number, FrameRequestCallback>();
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    const id = nextAnimationFrameId++;
    animationFrames.set(id, callback);
    return id;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => animationFrames.delete(id));
  let scrollTop = 0;
  const scrollTo = vi.fn((options: ScrollToOptions) => {
    if (options.behavior === 'instant' && options.top !== undefined) scrollTop = options.top;
  });
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
    scaleFactor: 1,
    appliedRevision: snapshot.revision,
    publishedRevision: snapshot.revision,
    pageEls: {
      0: {
        getBoundingClientRect: () => new DOMRect(0, 0, 600, 1200),
      },
    },
    scrollViewport: {
      getRect: () => new DOMRect(0, 0, 600, 400),
      getScrollTop: () => scrollTop,
      getScrollHeight: () => 1200,
      scrollTo,
    },
    safeDisplayZoom: () => 1,
    clientToLocal: vi.fn(() => null),
    captureSelectionViewportAnchor: vi.fn(() => void 0),
    captureViewportAnchorAt: vi.fn(() => void 0),
    trackedRangeForSnapshot: vi.fn((id: string, candidate: EditorSnapshot) => candidate.trackedRanges.find((range) => range.id === id)),
    requestPublication,
  } as unknown as Editor;
  return {
    advanceAnimation: (milliseconds = 16) => {
      animationTime += milliseconds;
      const callbacks = [...animationFrames.values()];
      animationFrames.clear();
      for (const callback of callbacks) callback(animationTime);
    },
    animationFrameCount: () => animationFrames.size,
    editor,
    getScrollTop: () => scrollTop,
    requestPublication,
    setScrollTop: (value: number) => {
      scrollTop = value;
    },
    scrollTo,
    scope: new EditorScrollScope(editor, () => typewriterPreferences),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('EditorScrollScope', () => {
  const revealMetrics = {
    scrollTop: 0,
    clientHeight: 400,
    scrollHeight: 1200,
    targetTop: 500,
    targetBottom: 520,
  };

  it('lets external content extend the bottom padding without changing the visible area', () => {
    const snapshot = trackedSnapshot('unused', {
      page_idx: 0,
      rect: { x: 0, y: 0, width: 1, height: 20 },
    });
    const { requestPublication, scope } = setup(snapshot);

    scope.setBottomInset(40);
    requestPublication.mockClear();
    scope.setContentBottomOverflow(180);

    expect(scope.bottomPaddingFor(snapshot)).toBe(220);
    expect(scope.visibleArea.bottomInset).toBe(40);
    expect(requestPublication).toHaveBeenCalledOnce();

    scope.setContentBottomOverflow(180);

    expect(requestPublication).toHaveBeenCalledOnce();

    scope.setContentBottomOverflow(0);

    expect(scope.bottomPaddingFor(snapshot)).toBe(40);
  });

  it('uses typewriter reveal for a collapsed current selection caret', () => {
    const rect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 20 } };
    const snapshot = selectionSnapshot(true, rect);
    const { scope } = setup(snapshot, { enabled: true, position: 0.5 });
    const request = scope.declare({ target: { type: 'current_selection_head' }, policy: 'typewriter' });

    expect(scope.resolveScrollTop(request, revealMetrics, snapshot)).toBe(310);
  });

  it('uses candidate unit-selection endpoint geometry for typewriter reveal and padding', () => {
    const publishedRect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 20 } };
    const candidateRect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 40 } };
    const publishedSnapshot = selectionSnapshot(true, publishedRect);
    const candidateSnapshot = selectionSnapshot(false, candidateRect);
    const { scope } = setup(publishedSnapshot, { enabled: true, position: 0.5 });
    const request = scope.declare({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
    const candidateMetrics = { ...revealMetrics, targetBottom: 540 };

    expect(scope.resolveScrollTop(request, candidateMetrics, candidateSnapshot)).toBe(320);
    expect(scope.resolvePreparationViewports(request, candidateMetrics, candidateSnapshot)).toEqual([{ top: 320, bottom: 720 }]);
    expect(scope.bottomPaddingFor(candidateSnapshot)).toBe(160);
  });

  it('uses typewriter reveal at the endpoint matching a keyboard-extended range head', () => {
    const headRect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 20 } };
    const baseSnapshot = selectionSnapshot(false, headRect);
    const fromRect = { page_idx: 0, rect: { x: 0, y: 20, width: 1, height: 20 } };
    if (!baseSnapshot.selectionEndpoints) throw new Error('range snapshot must have selection endpoints');
    const snapshot = {
      ...baseSnapshot,
      selectionEndpoints: { ...baseSnapshot.selectionEndpoints, from: fromRect },
    } as EditorSnapshot;
    const { scope } = setup(snapshot, { enabled: true, position: 0.5 });
    const request = scope.declare({ target: { type: 'current_selection_head' }, policy: 'typewriter' });

    expect(scope.resolveTargetRects(request.target, snapshot)).toEqual([headRect]);
    expect(scope.resolveScrollTop(request, revealMetrics, snapshot)).toBe(310);
  });

  it('keeps an explicit cursor-guard current-selection-head request out of typewriter mode', () => {
    const rect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 20 } };
    const snapshot = selectionSnapshot(false, rect);
    const { scope } = setup(snapshot, { enabled: true, position: 0.5 });
    const request = scope.declare({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });

    expect(scope.resolveScrollTop(request, revealMetrics, snapshot)).toBe(180);
  });

  it('falls back to cursor guard when typewriter is requested for a tracked item', () => {
    const rect = { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 20 } };
    const snapshot = selectionSnapshot(true, rect);
    const { scope } = setup(snapshot, { enabled: true, position: 0.5 });
    const request = scope.declare({ target: { type: 'tracked_item', id: 'unused' }, policy: 'typewriter' });

    expect(scope.resolveScrollTop(request, revealMetrics, snapshot)).toBe(180);
    expect(scope.resolvePreparationViewports(request, revealMetrics, snapshot)).toEqual([
      { top: 440, bottom: 840 },
      { top: 180, bottom: 580 },
    ]);
  });

  it('top-aligns an oversized tracked result below the viewport', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 1000, width: 1, height: 500 },
    });
    const { scope } = setup(snapshot);
    const request = scope.declare({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });

    expect(
      scope.resolveScrollTop(
        request,
        { scrollTop: 300, clientHeight: 400, scrollHeight: 2000, targetTop: 1000, targetBottom: 1500 },
        snapshot,
      ),
    ).toBe(940);
  });

  it('defaults request behavior to instant independently from the reveal policy', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 1000, width: 1, height: 500 },
    });
    const { scope } = setup(snapshot);

    const request = scope.declare({
      target: { type: 'tracked_item', id: 'target' },
      policy: 'reveal',
    });

    expect(request.behavior).toBe('instant');
  });

  it('accepts smooth behavior independently from the reveal policy', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 1000, width: 1, height: 500 },
    });
    const { scope } = setup(snapshot);

    const request = scope.declare({
      target: { type: 'tracked_item', id: 'target' },
      policy: 'reveal',
      behavior: 'smooth',
    });

    expect(request.behavior).toBe('smooth');
  });

  it('applies a smooth request immediately when reduced motion is preferred', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { animationFrameCount, scope, scrollTo } = setup(snapshot);
    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    const request = scope.declare({
      target: { type: 'tracked_item', id: 'target' },
      policy: 'reveal',
      behavior: 'smooth',
    });
    scope.bind(request, snapshot.revision);

    expect(scope.resolvePreparationViewports(request, revealMetrics, snapshot)).not.toEqual([]);
    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 580, behavior: 'instant' });
    expect(animationFrameCount()).toBe(0);
    expect(scope.pendingRequest).toBeNull();
  });

  it('applies the cursor guard to a compact pointer selection head', () => {
    const rect = { page_idx: 0, rect: { x: 0, y: 350, width: 1, height: 20 } };
    const snapshot = selectionSnapshot(true, rect);
    const { scope } = setup(snapshot);
    const request = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });

    expect(scope.resolveTargetRects(request.target, snapshot)).toEqual([rect]);
    expect(
      scope.resolveScrollTop(request, { scrollTop: 0, clientHeight: 400, scrollHeight: 1200, targetTop: 350, targetBottom: 370 }, snapshot),
    ).toBe(30);
    expect(
      scope.resolvePreparationViewports(
        request,
        { scrollTop: 0, clientHeight: 400, scrollHeight: 1200, targetTop: 350, targetBottom: 370 },
        snapshot,
      ),
    ).toContainEqual({ top: 30, bottom: 430 });
  });

  it('keeps an oversized pointer selection when one cursor margin is visible inside the guard', () => {
    const snapshot = selectionSnapshot(false, { page_idx: 0, rect: { x: 0, y: 280, width: 1, height: 500 } });
    const { scope } = setup(snapshot);
    const request = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });

    expect(
      scope.resolveScrollTop(request, { scrollTop: 0, clientHeight: 400, scrollHeight: 1200, targetTop: 280, targetBottom: 780 }, snapshot),
    ).toBeNull();
  });

  it('reveals one cursor margin of an oversized pointer selection entering from below', () => {
    const snapshot = selectionSnapshot(false, { page_idx: 0, rect: { x: 0, y: 310, width: 1, height: 500 } });
    const { scope } = setup(snapshot);
    const request = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });
    const metrics = { scrollTop: 0, clientHeight: 400, scrollHeight: 1200, targetTop: 310, targetBottom: 810 };

    expect(scope.resolveScrollTop(request, metrics, snapshot)).toBe(30);
    expect(scope.resolvePreparationViewports(request, metrics, snapshot)).toContainEqual({ top: 30, bottom: 430 });
  });

  it('reveals one cursor margin of an oversized pointer selection entering from above', () => {
    const snapshot = selectionSnapshot(false, { page_idx: 0, rect: { x: 0, y: 0, width: 1, height: 500 } });
    const { scope } = setup(snapshot);
    const request = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });
    const metrics = { scrollTop: 410, clientHeight: 400, scrollHeight: 1200, targetTop: 0, targetBottom: 500 };

    expect(scope.resolveScrollTop(request, metrics, snapshot)).toBe(380);
    expect(scope.resolvePreparationViewports(request, metrics, snapshot)).toContainEqual({ top: 380, bottom: 780 });
  });

  it('keeps pointer selection reveals eligible across later revisions', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 0, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const request = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });
    expect(scope.bind(request, 7)).toBe(true);

    expect(scope.activateForRevision(6)).toBeNull();
    expect(scope.activateForRevision(7)).toBe(request);
    expect(scope.activateForRevision(8)).toBe(request);
    expect(scope.pendingRequest).toBe(request);
  });

  it('keeps a declared reveal ineligible until the installing revision binds it', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const request = scope.declare({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });

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

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' }, admission);
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
    const { advanceAnimation, animationFrameCount, scope } = setup(snapshot);

    const presentation = scope.scrollIntoView({
      target: { type: 'tracked_item', id: 'target' },
      policy: 'reveal',
      behavior: 'smooth',
    });
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
    expect(scope.pendingRequest).toBe(request);
    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();
    await expect(presentation).resolves.toBeUndefined();
  });

  it('re-resolves current-selection and tracked-item targets from newer eligible revisions', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const tracked = scope.declare({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });
    expect(scope.bind(tracked, 7)).toBe(true);
    expect(scope.activateForRevision(8)).toBe(tracked);

    const selection = scope.declare({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    expect(scope.bind(selection, 7)).toBe(true);
    expect(scope.activateForRevision(8)).toBe(selection);
  });

  it('uses live tracked geometry when document edits only move the range', () => {
    const staleRect = {
      page_idx: 1,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    };
    const liveRect = {
      page_idx: 3,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    };
    const snapshot = trackedSnapshot('target', staleRect);
    const { editor, scope } = setup(snapshot);
    const range = snapshot.trackedRanges[0];
    if (!range) throw new Error('Expected a tracked range');
    editor.trackedRangeForSnapshot = vi.fn(() => ({ ...range, rects: [liveRect] }));

    expect(scope.resolveTargetRects({ type: 'tracked_item', id: 'target' }, snapshot)).toEqual([liveRect]);
  });

  it('completes a superseded reveal without applying it', async () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope } = setup(snapshot);
    const old = scope.declare({
      target: { type: 'current_selection_head' },
      policy: 'pointer_cursor_guard',
    });
    const oldPresentation = old.presentation;

    scope.declare({ target: { type: 'tracked_item', id: 'new' }, policy: 'reveal' });

    await expect(oldPresentation).resolves.toBeUndefined();
  });

  it('cancels a pending automatic reveal when direct manipulation takes control', async () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { requestPublication, scope } = setup(snapshot);
    const request = scope.declare({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });
    const presentation = request.presentation;
    requestPublication.mockClear();

    scope.cancel();

    await expect(presentation).resolves.toBeUndefined();
    expect(scope.pendingRequest).toBeNull();
    expect(requestPublication).toHaveBeenCalledOnce();
  });

  it('cancels the custom smooth scroll when direct manipulation takes control', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { advanceAnimation, animationFrameCount, scope, scrollTo } = setup(snapshot);
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');
    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);

    advanceAnimation();
    advanceAnimation();
    const callsBeforeCancel = scrollTo.mock.calls.length;
    scope.cancel();
    advanceAnimation();

    expect(animationFrameCount()).toBe(0);
    expect(scrollTo).toHaveBeenCalledTimes(callsBeforeCancel);
    expect(scrollTo.mock.calls.every(([options]) => options.behavior === 'instant')).toBe(true);
  });

  it('keeps the existing latest-request-wins reveal contract', () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { requestPublication, scope } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'old' }, policy: 'reveal', behavior: 'smooth' });
    const old = scope.pendingRequest;
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'new' }, policy: 'reveal', behavior: 'smooth' });

    expect(scope.pendingRequest?.target).toEqual({ type: 'tracked_item', id: 'new' });
    expect(requestPublication).toHaveBeenCalledTimes(2);
    expect(old).not.toBe(scope.pendingRequest);
  });

  it('applies only the request accepted with the matching publication', () => {
    const snapshot = trackedSnapshot('new', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { advanceAnimation, animationFrameCount, requestPublication, scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'old' }, policy: 'reveal', behavior: 'smooth' });
    const old = scope.pendingRequest;
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'new' }, policy: 'reveal', behavior: 'smooth' });
    const current = scope.pendingRequest;
    if (!old || !current) throw new Error('Expected both scroll requests');
    requestPublication.mockClear();

    expect(scope.applyPending(old, snapshot, { type: 'scroll_to', y: 100 })).toBe(false);
    expect(scope.applyPending(current, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    expect(scope.applyPending(current, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    expect(scrollTo).not.toHaveBeenCalled();
    expect(animationFrameCount()).toBe(1);
    expect(scope.applyPending(current, snapshot, { type: 'scroll_to', y: 640 })).toBe(true);
    expect(scope.pendingRequest).toBe(current);
    expect(requestPublication).not.toHaveBeenCalled();
    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();
    expect(scrollTo).toHaveBeenLastCalledWith({ top: 640, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
    expect(requestPublication).toHaveBeenCalledOnce();
  });

  it('does not reshape an in-flight motion when the same target is published again', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const run = (republish: boolean) => {
      const { advanceAnimation, getScrollTop, scope } = setup(snapshot);
      const request = scope.declare({
        target: { type: 'tracked_item', id: 'target' },
        policy: 'reveal',
        behavior: 'smooth',
      });
      scope.bind(request, snapshot.revision);
      scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 });
      for (let frame = 0; frame < 8; frame++) advanceAnimation();
      if (republish) scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 });
      advanceAnimation();
      return getScrollTop();
    };

    expect(run(true)).toBeCloseTo(run(false), 8);
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

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
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
      policy: 'cursor_guard',
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

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 280 })).toBe(true);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 280, behavior: 'instant' });
  });

  it('converges a provisional selection reveal to the destination for its measured geometry', () => {
    const provisional = selectionSnapshot(false, {
      page_idx: 0,
      rect: { x: 0, y: 500, width: 1, height: 1 },
    });
    const measured = {
      ...selectionSnapshot(false, {
        page_idx: 0,
        rect: { x: 0, y: 500, width: 1, height: 500 },
      }),
      revision: 2,
    } as EditorSnapshot;
    const { editor, scope } = setup(provisional);
    const anchor = { type: 'node' as const, node: '1:1', offset_x: 0, offset_y: 0 };
    editor.captureSelectionViewportAnchor = vi.fn(() => ({
      identity: anchor,
      geometry: {
        point: { page_idx: 0, x: 0, y: 500.5 },
        rect: { page_idx: 0, rect: { x: 0, y: 500, width: 1, height: 1 } },
      },
    }));
    editor.resolveViewportAnchor = vi.fn((revision) => ({
      type: 'resolved' as const,
      geometry: {
        point: { page_idx: 0, x: 0, y: revision === provisional.revision ? 500.5 : 750 },
        rect: {
          page_idx: 0,
          rect: { x: 0, y: 500, width: 1, height: revision === provisional.revision ? 1 : 500 },
        },
      },
    }));

    scope.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');
    expect(scope.applyPending(request, provisional, { type: 'scroll_to', y: 161 })).toBe(true);

    expect(scope.prepareViewportAnchorPublication(measured)).toMatchObject({
      type: 'ready',
      targetScrollTop: 660,
    });
  });

  it('keeps a retained content anchor exactly attached when candidate geometry moves above it', () => {
    const current = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 190, width: 1, height: 20 },
    });
    const candidate = { ...current, revision: 2 };
    const { editor, scope } = setup(current);
    const anchor = { type: 'node' as const, node: '1:1', offset_x: 0, offset_y: 0 };
    let scrollTop = 100;
    const scrollTo = vi.fn((options: ScrollToOptions) => {
      scrollTop = options.top ?? scrollTop;
    });
    editor.scrollViewport = {
      ...editor.scrollViewport,
      getScrollTop: () => scrollTop,
      scrollTo,
    } as NonNullable<Editor['scrollViewport']>;
    editor.captureSelectionViewportAnchor = vi.fn(() => ({
      identity: anchor,
      geometry: {
        point: { page_idx: 0, x: 0, y: 200 },
        rect: { page_idx: 0, rect: { x: 0, y: 190, width: 1, height: 20 } },
      },
    }));
    editor.resolveViewportAnchor = vi.fn((revision) => ({
      type: 'resolved' as const,
      geometry: {
        point: { page_idx: 0, x: 0, y: revision === 1 ? 200 : 320 },
        rect: { page_idx: 0, rect: { x: 0, y: revision === 1 ? 190 : 310, width: 1, height: 20 } },
      },
    }));

    const publication = scope.prepareViewportAnchorPublication(candidate);

    expect(publication).toMatchObject({ type: 'ready', targetScrollTop: 220 });
    scope.applyViewportAnchorPublication(publication);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 220, behavior: 'instant' });
    expect(scrollTop).toBe(220);
  });

  it('attaches a changed selection without scrolling then guards it after the viewport shrinks', () => {
    const initial = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 90, width: 1, height: 20 },
    });
    const nextPosition = { node: 'text', offset: 1, affinity: 'downstream' as const };
    const nextRect = { page_idx: 0, rect: { x: 0, y: 340, width: 1, height: 20 } };
    const next = {
      ...selectionSnapshot(true, nextRect),
      revision: 2,
      selection: { anchor: nextPosition, head: nextPosition },
      selectionEndpoints: {
        from: nextRect,
        to: nextRect,
        from_position: nextPosition,
        to_position: nextPosition,
      },
    } as EditorSnapshot;
    const { editor, getScrollTop, scope, scrollTo } = setup(initial);
    const initialAnchor = { type: 'node' as const, node: 'selection-1', offset_x: 0, offset_y: 0 };
    const nextAnchor = { type: 'node' as const, node: 'selection-2', offset_x: 0, offset_y: 0 };
    const initialGeometry = {
      point: { page_idx: 0, x: 0, y: 100 },
      rect: { page_idx: 0, rect: { x: 0, y: 90, width: 1, height: 20 } },
    };
    const nextGeometry = {
      point: { page_idx: 0, x: 0, y: 350 },
      rect: nextRect,
    };
    let capture = { identity: initialAnchor, geometry: initialGeometry };
    editor.captureSelectionViewportAnchor = vi.fn(() => capture);
    editor.resolveViewportAnchor = vi.fn((_revision, identity) => ({
      type: 'resolved' as const,
      geometry: identity === initialAnchor ? initialGeometry : nextGeometry,
    }));

    expect(scope.prepareViewportAnchorPublication(initial).type).toBe('ready');
    capture = { identity: nextAnchor, geometry: nextGeometry };
    Object.assign(editor, {
      published: { snapshot: next, frames: editor.published?.frames ?? new Map() },
      publishedRevision: next.revision,
    });

    const publication = scope.prepareViewportAnchorPublication(next);
    scope.applyViewportAnchorPublication(publication);

    expect(scrollTo).not.toHaveBeenCalled();
    expect(getScrollTop()).toBe(0);

    scope.setVisibleArea({ topInset: 0, bottomInset: 100 });

    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 120, behavior: 'instant' });
    expect(getScrollTop()).toBe(120);
  });

  it('keeps the viewport center active for an initial selection outside the guard', () => {
    const initial = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 490, width: 1, height: 20 },
    });
    const { editor, scope } = setup(initial);
    const selection = { type: 'node' as const, node: 'selection', offset_x: 0, offset_y: 0 };
    const viewport = { type: 'node' as const, node: 'viewport', offset_x: 0, offset_y: 0 };
    const selectionGeometry = {
      point: { page_idx: 0, x: 0, y: 500 },
      rect: { page_idx: 0, rect: { x: 0, y: 490, width: 1, height: 20 } },
    };
    const viewportGeometry = { point: { page_idx: 0, x: 0, y: 200 }, rect: undefined };
    editor.captureSelectionViewportAnchor = vi.fn(() => ({ identity: selection, geometry: selectionGeometry }));
    editor.captureViewportAnchorAt = vi.fn(() => ({ identity: viewport, geometry: viewportGeometry }));
    editor.resolveViewportAnchor = vi.fn((_revision, identity) => ({
      type: 'resolved' as const,
      geometry: identity.node === selection.node ? selectionGeometry : viewportGeometry,
    }));

    expect(scope.prepareViewportAnchorPublication(initial)).toEqual({
      type: 'ready',
      geometry: { pointY: 200 },
      targetScrollTop: 0,
    });
  });

  it('adopts the viewport center without scrolling when selection is removed', () => {
    const initial = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 90, width: 1, height: 20 },
    });
    const candidate = {
      ...initial,
      revision: 2,
      cursor: undefined,
      selection: undefined,
      selectionEndpoints: undefined,
    } as EditorSnapshot;
    const { editor, scope, scrollTo } = setup(initial);
    const selection = { type: 'node' as const, node: 'selection', offset_x: 0, offset_y: 0 };
    const viewport = { type: 'node' as const, node: 'viewport', offset_x: 0, offset_y: 0 };
    const selectionGeometry = {
      point: { page_idx: 0, x: 0, y: 100 },
      rect: { page_idx: 0, rect: { x: 0, y: 90, width: 1, height: 20 } },
    };
    const viewportGeometry = { point: { page_idx: 0, x: 0, y: 500 }, rect: undefined };
    let capture: { identity: typeof selection; geometry: typeof selectionGeometry } | undefined = {
      identity: selection,
      geometry: selectionGeometry,
    };
    editor.captureSelectionViewportAnchor = vi.fn(() => capture);
    editor.captureViewportAnchorAt = vi.fn(() => ({ identity: viewport, geometry: viewportGeometry }));
    editor.resolveViewportAnchor = vi.fn((_revision, identity) => ({
      type: 'resolved' as const,
      geometry: identity.node === selection.node ? selectionGeometry : viewportGeometry,
    }));

    expect(scope.prepareViewportAnchorPublication(initial).type).toBe('ready');
    capture = undefined;

    expect(scope.prepareViewportAnchorPublication(candidate)).toEqual({
      type: 'ready',
      geometry: { pointY: 500 },
      targetScrollTop: 0,
    });
    expect(scrollTo).not.toHaveBeenCalled();
  });

  it('keeps existing anchors and publishes while candidate selection geometry is unavailable', () => {
    const initial = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 90, width: 1, height: 20 },
    });
    const candidate = { ...initial, revision: 2 } as EditorSnapshot;
    const { editor, scope } = setup(initial);
    const selection = { type: 'node' as const, node: 'selection', offset_x: 0, offset_y: 0 };
    const geometry = {
      point: { page_idx: 0, x: 0, y: 100 },
      rect: { page_idx: 0, rect: { x: 0, y: 90, width: 1, height: 20 } },
    };
    let capture: { identity: typeof selection; geometry: typeof geometry } | undefined = { identity: selection, geometry };
    editor.captureSelectionViewportAnchor = vi.fn(() => capture);
    editor.resolveViewportAnchor = vi.fn(() => ({ type: 'resolved' as const, geometry }));

    expect(scope.prepareViewportAnchorPublication(initial).type).toBe('ready');
    capture = undefined;

    expect(scope.prepareViewportAnchorPublication(candidate).type).toBe('ready');
    capture = { identity: { ...selection }, geometry };
    expect(scope.prepareViewportAnchorPublication(initial)).toMatchObject({
      type: 'ready',
      geometry: { pointY: 100 },
    });
  });

  it('does not withhold a publication when a live anchor has no candidate geometry', () => {
    const current = selectionSnapshot(true, {
      page_idx: 0,
      rect: { x: 0, y: 190, width: 1, height: 20 },
    });
    const candidate = { ...current, revision: 2 };
    const { editor, scope } = setup(current);
    const selection = { type: 'node' as const, node: '1:1', offset_x: 0, offset_y: 0 };
    const viewport = { type: 'node' as const, node: '2:1', offset_x: 0, offset_y: 0 };
    const capturedGeometry = {
      point: { page_idx: 0, x: 0, y: 200 },
      rect: { page_idx: 0, rect: { x: 0, y: 190, width: 1, height: 20 } },
    };
    editor.captureSelectionViewportAnchor = vi.fn(() => ({ identity: selection, geometry: capturedGeometry }));
    editor.captureViewportAnchorAt = vi.fn(() => ({ identity: viewport, geometry: capturedGeometry }));
    editor.resolveViewportAnchor = vi.fn((revision) =>
      revision === current.revision ? { type: 'resolved' as const, geometry: capturedGeometry } : { type: 'not_laid_out' as const },
    );

    expect(scope.prepareViewportAnchorPublication(current).type).toBe('ready');
    expect(scope.prepareViewportAnchorPublication(candidate)).toEqual({
      type: 'ready',
      geometry: null,
      targetScrollTop: null,
    });
  });

  it('clamps an unanchored publication to the candidate scroll extent', () => {
    const current = trackedSnapshot('target', {
      page_idx: 1,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const candidate = {
      ...current,
      revision: 2,
      pageSizes: [{ width: 600, height: 300 }],
    } as EditorSnapshot;
    const { editor, getScrollTop, scope, scrollTo, setScrollTop } = setup(current);
    editor.scrollRootEl = {} as HTMLElement;
    setScrollTop(800);

    const publication = scope.prepareViewportAnchorPublication(candidate);

    expect(publication).toEqual({ type: 'ready', geometry: null, targetScrollTop: 0 });
    scope.applyViewportAnchorPublication(publication);
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 0, behavior: 'instant' });
    expect(getScrollTop()).toBe(0);
  });

  it('does not clamp an unanchored publication to the editor extent in a window scroller', () => {
    const current = trackedSnapshot('target', {
      page_idx: 1,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const candidate = {
      ...current,
      revision: 2,
      pageSizes: [{ width: 600, height: 300 }],
    } as EditorSnapshot;
    const { editor, scope, setScrollTop } = setup(current);
    editor.scrollRootEl = null;
    setScrollTop(800);

    expect(scope.prepareViewportAnchorPublication(candidate)).toEqual({
      type: 'ready',
      geometry: null,
      targetScrollTop: null,
    });
  });

  it('translates an in-flight smooth reveal across viewport-anchor reconciliation', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { advanceAnimation, animationFrameCount, scope, scrollTo } = setup(snapshot);
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    advanceAnimation();
    advanceAnimation();
    scope.applyViewportAnchorPublication({
      type: 'ready',
      geometry: { pointY: 320 },
      targetScrollTop: 220,
    });
    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);

    expect(scrollTo).toHaveBeenCalledWith({ top: 220, behavior: 'instant' });
    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();
    expect(scrollTo).toHaveBeenLastCalledWith({ top: 580, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
  });

  it('keeps a smooth reveal pending until the latest applied geometry is published', async () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { advanceAnimation, animationFrameCount, editor, requestPublication, scope, scrollTo } = setup(snapshot);
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');
    const presentation = request.presentation;

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    const candidate = { ...snapshot, revision: 2 } as EditorSnapshot;
    Object.assign(editor, { appliedSnapshot: candidate, appliedRevision: candidate.revision });

    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();

    expect(scrollTo).toHaveBeenLastCalledWith({ top: 580, behavior: 'instant' });
    expect(scope.pendingRequest).toBe(request);
    expect(requestPublication).toHaveBeenCalled();

    Object.assign(editor, {
      published: { snapshot: candidate, frames: editor.published?.frames ?? new Map() },
      publishedRevision: candidate.revision,
    });
    expect(scope.applyPending(request, candidate, { type: 'scroll_to', y: 780 })).toBe(true);
    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();

    expect(scrollTo).toHaveBeenLastCalledWith({ top: 780, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
    await expect(presentation).resolves.toBeUndefined();
  });

  it('finishes an in-flight smooth reveal when a later publication resolves it as no-op', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { scope, scrollTo } = setup(snapshot);
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    scope.applyViewportAnchorPublication({
      type: 'ready',
      geometry: { pointY: 320 },
      targetScrollTop: 220,
    });
    expect(scope.applyPending(request, snapshot, { type: 'no_scroll' })).toBe(true);

    expect(scrollTo).toHaveBeenCalledExactlyOnceWith({ top: 220, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
  });

  it('keeps a non-selection reveal anchored near its destination after the visible area shrinks', () => {
    const selectionRect = {
      page_idx: 0,
      rect: { x: 0, y: 90, width: 1, height: 20 },
    };
    const targetRect = {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    };
    const snapshot = {
      ...selectionSnapshot(true, selectionRect),
      trackedRanges: [{ id: 'target', rects: [targetRect] }],
    } as EditorSnapshot;
    const { editor, getScrollTop, scope, scrollTo } = setup(snapshot);
    const selection = { type: 'node' as const, node: 'selection', offset_x: 0, offset_y: 0 };
    const viewport = { type: 'node' as const, node: 'viewport', offset_x: 0, offset_y: 0 };
    const selectionGeometry = {
      point: { page_idx: 0, x: 0, y: 100 },
      rect: selectionRect,
    };
    const viewportGeometry = {
      point: { page_idx: 0, x: 0, y: 700 },
      rect: undefined,
    };
    editor.captureSelectionViewportAnchor = vi.fn(() => ({ identity: selection, geometry: selectionGeometry }));
    editor.captureViewportAnchorAt = vi.fn(() => ({ identity: viewport, geometry: viewportGeometry }));
    editor.resolveViewportAnchor = vi.fn((_revision, identity) => ({
      type: 'resolved' as const,
      geometry: identity === selection ? selectionGeometry : viewportGeometry,
    }));

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');
    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    scrollTo.mockClear();

    scope.visibleArea = { topInset: 0, bottomInset: 100 };
    scope.reconcileViewportResize();

    expect(scrollTo).not.toHaveBeenCalled();
    expect(getScrollTop()).toBe(580);
  });

  it('finishes an in-flight smooth reveal at its current position when a no-op publication is within scroll tolerance', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 0,
      rect: { x: 0, y: 900, width: 1, height: 20 },
    });
    const { getScrollTop, scope, scrollTo, setScrollTop } = setup(snapshot);
    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a pending reveal');

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 580 })).toBe(true);
    setScrollTop(579.25);
    expect(scope.applyPending(request, snapshot, { type: 'no_scroll' })).toBe(true);

    expect(getScrollTop()).toBe(579.25);
    expect(scrollTo).not.toHaveBeenCalled();
    expect(scope.pendingRequest).toBeNull();
  });

  it('starts a smooth reveal without requiring a frame for the target page', () => {
    const snapshot = trackedSnapshot('target', {
      page_idx: 1,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const { advanceAnimation, animationFrameCount, editor, scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'target' }, policy: 'reveal', behavior: 'smooth' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');

    editor.published = {
      snapshot,
      frames: new Map([[0, { revision: snapshot.revision, surfaceKey: 1, frameKey: 1, canvas: document.createElement('canvas') }]]),
    };
    editor.pageEls[1] = undefined;

    expect(scope.applyPending(request, snapshot, { type: 'scroll_to', y: 900 })).toBe(true);
    expect(scope.pendingRequest).toBe(request);
    expect(scrollTo).not.toHaveBeenCalled();
    for (let frame = 0; frame < 100 && animationFrameCount() > 0; frame++) advanceAnimation();
    expect(scrollTo).toHaveBeenLastCalledWith({ top: 800, behavior: 'instant' });
    expect(scope.pendingRequest).toBeNull();
  });

  it('completes an eligible reveal as a no-op when the matching snapshot has no target', async () => {
    const snapshot = trackedSnapshot('other', {
      page_idx: 0,
      rect: { x: 0, y: 100, width: 1, height: 20 },
    });
    const { scope, scrollTo } = setup(snapshot);

    scope.scrollIntoView({ target: { type: 'tracked_item', id: 'missing' }, policy: 'reveal' });
    const request = scope.pendingRequest;
    if (!request) throw new Error('Expected a scroll request');
    const presentation = request.presentation;

    expect(scope.applyPending(request, snapshot, { type: 'no_scroll' })).toBe(true);
    await expect(presentation).resolves.toBeUndefined();
    expect(scope.pendingRequest).toBeNull();
    expect(scrollTo).not.toHaveBeenCalled();
  });
});
