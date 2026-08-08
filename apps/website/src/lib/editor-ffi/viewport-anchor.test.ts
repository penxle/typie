import { describe, expect, it } from 'vitest';
import { EditorViewportAnchorState } from './viewport-anchor';
import type { ViewportAnchor } from '@typie/editor-ffi/browser';

const identity: ViewportAnchor = { type: 'node', node: '1:1', offset_x: 0, offset_y: 0 };
const viewportIdentity: ViewportAnchor = { type: 'node', node: '2:1', offset_x: 0, offset_y: 0 };
const visibleArea = { topInset: 0, bottomInset: 0 };

describe('EditorViewportAnchorState', () => {
  it('keeps the anchor at its exact attached viewport point across publication', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointY: 200 }, 100);

    expect(state.publicationScroll({ pointY: 320 }, 100, 500)).toBe(220);
  });

  it('does not move when geometry changes below the anchor', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointY: 200 }, 100);

    expect(state.publicationScroll({ pointY: 200 }, 100, 500)).toBe(100);
  });

  it('retains a directly scrolled anchor inside the cursor guard and rejects it outside', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointY: 200, rect: { top: 190, bottom: 210 } };
    state.attach(identity, geometry, 100);

    expect(state.canRetainAfterDirectScroll(geometry, 80, 300, visibleArea)).toBe(true);
    expect(state.canRetainAfterDirectScroll(geometry, 170, 300, visibleArea)).toBe(false);
  });

  it('uses the point when the anchor rect is taller than the guard', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointY: 150, rect: { top: 0, bottom: 1000 } };
    state.attach(identity, geometry, 0);

    expect(state.canRetainAfterDirectScroll(geometry, 0, 300, visibleArea)).toBe(true);
    expect(state.resizeScroll(geometry, 0, 300, 1000, visibleArea)).toBe(0);
  });

  it('moves minimally after resize pushes the anchor outside the cursor guard', () => {
    const state = new EditorViewportAnchorState();
    const geometry = { pointY: 260, rect: { top: 250, bottom: 270 } };
    state.attach(identity, geometry, 100);

    expect(state.resizeScroll(geometry, 100, 300, 1000, { topInset: 0, bottomInset: 100 })).toBe(130);
  });

  it('records the achieved attachment after publication clamping', () => {
    const state = new EditorViewportAnchorState();
    state.attach(identity, { pointY: 200 }, 100);
    const candidate = { pointY: 800 };

    const clamped = state.publicationScroll(candidate, 100, 500);
    state.acceptGeometry(candidate, clamped);

    expect(clamped).toBe(500);
    expect(state.pointAttachmentY).toBe(300);
  });

  it('finishes a provisional selection reveal where the measured rect would have revealed initially', () => {
    const state = new EditorViewportAnchorState();
    const provisional = { pointY: 500.5, rect: { top: 500, bottom: 501 } };
    const measured = { pointY: 600, rect: { top: 500, bottom: 700 } };
    state.attachSelection(identity, provisional, 161);

    expect(state.publicationRevealScroll(measured, 161, 400, 1000, visibleArea)).toBe(360);
  });

  it('reactivates a preferred selection rect after direct scrolling returns it inside the cursor guard', () => {
    const state = new EditorViewportAnchorState();
    const selectionGeometry = { pointY: 200, rect: { top: 190, bottom: 210 } };
    state.attachSelection(identity, selectionGeometry, 100);
    state.attachViewport(viewportIdentity, { pointY: 500 }, 350);

    expect(state.tryReactivatePreferredSelection(selectionGeometry, 120, 300, visibleArea)).toBe(true);
    expect(state.identity).toBe(identity);
    expect(state.pointAttachmentY).toBe(80);
  });

  it('does not reactivate a preferred selection without a compact rect inside the guard', () => {
    const state = new EditorViewportAnchorState();
    state.attachSelection(identity, { pointY: 200, rect: { top: 0, bottom: 1000 } }, 100);
    state.attachViewport(viewportIdentity, { pointY: 500 }, 350);

    expect(state.tryReactivatePreferredSelection({ pointY: 200, rect: { top: 0, bottom: 1000 } }, 100, 300, visibleArea)).toBe(false);
    expect(state.tryReactivatePreferredSelection({ pointY: 200 }, 100, 300, visibleArea)).toBe(false);
    expect(state.identity).toBe(viewportIdentity);
  });
});
