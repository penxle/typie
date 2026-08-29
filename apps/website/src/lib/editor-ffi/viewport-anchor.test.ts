import { describe, expect, it } from 'vitest';
import { EditorViewportAnchorState, resolveViewportAnchorGeometry, viewportCenterAnchorPoint } from './viewport-anchor';
import type { ViewportAnchor } from '@typie/editor-ffi/browser';
import type { EditorSnapshot } from './editor.svelte';

const identity: ViewportAnchor = { type: 'node', node: '1:1', offset_x: 0, offset_y: 0 };
const viewportIdentity: ViewportAnchor = { type: 'node', node: '2:1', offset_x: 0, offset_y: 0 };
const visibleArea = { topInset: 0, bottomInset: 0 };

describe('EditorViewportAnchorState', () => {
  it('resolves the viewport center on both axes without DOM geometry', () => {
    const snapshot = {
      pageSizes: [{ width: 600, height: 1200 }],
    } as EditorSnapshot;

    expect(
      viewportCenterAnchorPoint(
        snapshot,
        { pages: [{ page: 0, left: 100, top: 20, bottom: 1220 }], zoom: 2 },
        { scrollLeft: 300, scrollTop: 220, clientWidth: 400, clientHeight: 200 },
        { topInset: 20, bottomInset: 40 },
      ),
    ).toEqual({ page_idx: 0, x: 200, y: 145 });
  });

  it('uses the gap midpoint and clamps viewport centers to page bounds', () => {
    const snapshot = {
      pageSizes: [
        { width: 600, height: 100 },
        { width: 600, height: 100 },
      ],
    } as EditorSnapshot;
    const layout = {
      pages: [
        { page: 0, left: 0, top: 0, bottom: 100 },
        { page: 1, left: 0, top: 124, bottom: 224 },
      ],
      zoom: 1,
    };

    const metrics = (scrollTop: number) => ({ scrollLeft: 0, scrollTop, clientWidth: 600, clientHeight: 20 });
    expect(viewportCenterAnchorPoint(snapshot, layout, metrics(100), visibleArea)).toMatchObject({ page_idx: 0, y: 100 });
    expect(viewportCenterAnchorPoint(snapshot, layout, metrics(104), visibleArea)).toMatchObject({ page_idx: 1, y: 0 });
    expect(viewportCenterAnchorPoint(snapshot, layout, metrics(300), visibleArea)).toMatchObject({ page_idx: 1, y: 100 });
  });

  it('resolves page-local points through the displayed page origin and zoom on both axes', () => {
    const resolved = resolveViewportAnchorGeometry(
      { point: { page_idx: 0, x: 40, y: 50 }, rect: undefined },
      { pages: [{ page: 0, left: 120, top: 300, bottom: 900 }], zoom: 1.5 },
    );

    expect(resolved).toEqual({ pointX: 180, pointY: 375 });
  });

  it('keeps the anchor at its exact attached viewport point across publication', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointX: 100, pointY: 200 }, { left: 20, top: 100 });

    expect(state.publicationScroll({ pointX: 260, pointY: 320 }, { left: 20, top: 100 }, { left: 500, top: 500 })).toEqual({
      scroll: { left: 180, top: 220 },
      attachmentAchieved: true,
    });
  });

  it('does not move when geometry changes below the anchor', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointX: 0, pointY: 200 }, { left: 0, top: 100 });

    expect(state.publicationScroll({ pointX: 0, pointY: 200 }, { left: 0, top: 100 }, { left: 0, top: 500 })).toEqual({
      scroll: { left: 0, top: 100 },
      attachmentAchieved: true,
    });
  });

  it('retains a directly scrolled anchor inside the cursor guard and rejects it outside', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointX: 0, pointY: 200, rect: { top: 190, bottom: 210 } };
    state.attach(identity, geometry, { left: 0, top: 100 });

    expect(state.canRetainAfterDirectScroll(geometry, 80, 300, visibleArea)).toBe(true);
    expect(state.canRetainAfterDirectScroll(geometry, 170, 300, visibleArea)).toBe(false);
  });

  it('uses the point when the anchor rect is taller than the guard', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointX: 0, pointY: 150, rect: { top: 0, bottom: 1000 } };
    state.attach(identity, geometry, { left: 0, top: 0 });

    expect(state.canRetainAfterDirectScroll(geometry, 0, 300, visibleArea)).toBe(true);
    expect(state.resizeScroll(geometry, 0, 300, 1000, visibleArea)).toBe(0);
  });

  it('moves minimally after resize pushes the anchor outside the cursor guard', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointX: 0, pointY: 260, rect: { top: 250, bottom: 270 } };
    state.attach(identity, geometry, { left: 0, top: 100 });

    expect(state.resizeScroll(geometry, 100, 300, 1000, { topInset: 0, bottomInset: 100 })).toBe(130);
  });

  it('preserves the desired attachment until publication bounds can reach it', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointX: 200, pointY: 200 }, { left: 100, top: 100 });
    const candidate = { pointX: 800, pointY: 800 };

    const constrained = state.publicationScroll(candidate, { left: 100, top: 100 }, { left: 500, top: 500 });

    expect(constrained).toEqual({ scroll: { left: 500, top: 500 }, attachmentAchieved: false });
    expect(state.publicationScroll(candidate, constrained.scroll, { left: 1000, top: 1000 })).toEqual({
      scroll: { left: 700, top: 700 },
      attachmentAchieved: true,
    });
    expect(state.pointAttachmentX).toBe(100);
    expect(state.pointAttachmentY).toBe(100);
  });

  it('updates a zoom attachment without replacing its stable viewport identity', () => {
    const state = new EditorViewportAnchorState();
    state.attachViewport(viewportIdentity, { pointX: 200, pointY: 300 }, { left: 100, top: 200 });

    expect(state.reattachViewport({ pointX: 320, pointY: 440 }, { left: 220, top: 340 })).toBe(true);

    expect(state.identity).toBe(viewportIdentity);
    expect(state.viewportAttachment).toEqual({ identity: viewportIdentity, focalX: 100, focalY: 100 });
    expect(state.publicationScroll({ pointX: 400, pointY: 500 }, { left: 220, top: 340 }, { left: 500, top: 500 })).toEqual({
      scroll: { left: 300, top: 400 },
      attachmentAchieved: true,
    });
  });

  it('finishes a provisional selection reveal where the measured rect would have revealed initially', () => {
    const state = new EditorViewportAnchorState();
    const provisional = { pointX: 0, pointY: 500.5, rect: { top: 500, bottom: 501 } };
    const measured = { pointX: 0, pointY: 600, rect: { top: 500, bottom: 700 } };
    state.attachSelection(identity, provisional, { left: 0, top: 161 });

    expect(state.publicationRevealScroll(measured, 161, 400, 1000, visibleArea)).toEqual({
      scroll: { left: 0, top: 360 },
      attachmentAchieved: true,
    });
  });

  it('reactivates a preferred selection rect after direct scrolling returns it inside the cursor guard', () => {
    const state = new EditorViewportAnchorState();
    const selectionGeometry = { pointX: 0, pointY: 200, rect: { top: 190, bottom: 210 } };
    state.attachSelection(identity, selectionGeometry, { left: 0, top: 100 });
    state.attachViewport(viewportIdentity, { pointX: 0, pointY: 500 }, { left: 0, top: 350 });

    expect(state.tryReactivatePreferredSelection(selectionGeometry, { left: 0, top: 120 }, 300, visibleArea)).toBe(true);
    expect(state.identity).toBe(identity);
    expect(state.pointAttachmentY).toBe(80);
  });

  it('keeps the directly scrolled horizontal position when the preferred selection remains visible', () => {
    const state = new EditorViewportAnchorState();
    const selectionGeometry = { pointX: 200, pointY: 200, rect: { top: 190, bottom: 210 } };
    state.attachSelection(identity, selectionGeometry, { left: 0, top: 100 });

    expect(state.tryReactivatePreferredSelection(selectionGeometry, { left: 120, top: 100 }, 300, visibleArea)).toBe(true);
    expect(state.pointAttachmentX).toBe(80);
    expect(state.publicationScroll(selectionGeometry, { left: 120, top: 100 }, { left: 500, top: 500 }).scroll.left).toBe(120);
  });

  it('does not reactivate a preferred selection without a compact rect inside the guard', () => {
    const state = new EditorViewportAnchorState();
    state.attachSelection(identity, { pointX: 0, pointY: 200, rect: { top: 0, bottom: 1000 } }, { left: 0, top: 100 });
    state.attachViewport(viewportIdentity, { pointX: 0, pointY: 500 }, { left: 0, top: 350 });

    expect(
      state.tryReactivatePreferredSelection(
        { pointX: 0, pointY: 200, rect: { top: 0, bottom: 1000 } },
        { left: 0, top: 100 },
        300,
        visibleArea,
      ),
    ).toBe(false);
    expect(state.tryReactivatePreferredSelection({ pointX: 0, pointY: 200 }, { left: 0, top: 100 }, 300, visibleArea)).toBe(false);
    expect(state.identity).toBe(viewportIdentity);
  });

  it('compares selection adoption by stable anchor identity', () => {
    const state = new EditorViewportAnchorState();
    state.attachSelection(identity, { pointX: 0, pointY: 200 }, { left: 0, top: 100 });

    expect(state.needsSelectionAdoption({ ...identity })).toBe(false);
    expect(state.needsSelectionAdoption(viewportIdentity)).toBe(true);
  });

  it('updates the preferred selection without replacing the active viewport anchor', () => {
    const state = new EditorViewportAnchorState();
    state.attachViewport(viewportIdentity, { pointX: 0, pointY: 500 }, { left: 0, top: 350 });

    state.adoptSelection(identity, { pointX: 0, pointY: 200 }, { left: 0, top: 350 }, 300, visibleArea, true);

    expect(state.identity).toBe(viewportIdentity);
    expect(state.preferredSelectionIdentity).toBe(identity);
  });
});
