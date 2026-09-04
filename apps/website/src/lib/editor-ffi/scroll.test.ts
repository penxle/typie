import { describe, expect, it } from 'vitest';
import {
  resolveGuardedScrollTop,
  resolveKeepVisibleBottomPadding,
  resolveScrollPastEndBottomPadding,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';

describe('resolveGuardedScrollTop', () => {
  it('keeps the target inside the guarded visible area with insets', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 100,
        clientHeight: 400,
        scrollHeight: 1000,
        targetTop: 430,
        targetBottom: 450,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(130);

    expect(
      resolveGuardedScrollTop({
        scrollTop: 100,
        clientHeight: 400,
        scrollHeight: 1000,
        targetTop: 150,
        targetBottom: 170,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(80);
  });

  it('returns null when the target is already visible', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 100,
        clientHeight: 400,
        scrollHeight: 1000,
        targetTop: 220,
        targetBottom: 240,
      }),
    ).toBeNull();
  });

  it('aligns an oversized target below the viewport bottom to the lower cursor guard', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 1000,
        targetBottom: 1500,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(1180);
  });

  it('does not scroll when an oversized target already covers the guarded visible area', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 250,
        targetBottom: 800,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBeNull();
  });

  it('does not scroll when an oversized target exactly meets either guard edge and covers the other', () => {
    const metrics = {
      scrollTop: 300,
      clientHeight: 400,
      scrollHeight: 2000,
      visibleArea: { topInset: 10, bottomInset: 20 },
    };

    expect(resolveGuardedScrollTop({ ...metrics, targetTop: 370, targetBottom: 800 })).toBeNull();
    expect(resolveGuardedScrollTop({ ...metrics, targetTop: 100, targetBottom: 620 })).toBeNull();
  });

  it('aligns an oversized target to the violated guard edge even within the old slack', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 390,
        targetBottom: 900,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(580);
  });

  it('clamps an oversized cursor reveal at the document edge', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 10,
        clientHeight: 400,
        scrollHeight: 800,
        targetTop: 0,
        targetBottom: 251,
        visibleArea: { topInset: 30, bottomInset: 0 },
      }),
    ).toBe(0);
  });

  it('aligns an oversized target above the viewport top to the upper cursor guard', () => {
    expect(
      resolveGuardedScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 0,
        targetBottom: 400,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(0);
  });
});

describe('resolveTypewriterScrollTop', () => {
  it('aligns the target top to the configured typewriter position', () => {
    expect(
      resolveTypewriterScrollTop({
        scrollTop: 0,
        clientHeight: 500,
        scrollHeight: 2000,
        targetTop: 800,
        targetBottom: 820,
        visibleArea: { topInset: 10, bottomInset: 30 },
        position: 0.5,
      }),
    ).toBe(570);
  });

  it('clamps to the current max scroll extent', () => {
    expect(
      resolveTypewriterScrollTop({
        scrollTop: 1000,
        clientHeight: 500,
        scrollHeight: 2000,
        targetTop: 1980,
        targetBottom: 2000,
        position: 0.5,
      }),
    ).toBe(1500);
  });

  it('falls back to the lower cursor guard for an oversized target below the viewport', () => {
    expect(
      resolveTypewriterScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 1000,
        targetBottom: 1500,
        visibleArea: { topInset: 10, bottomInset: 20 },
        position: 0.5,
      }),
    ).toBe(1180);
  });

  it('keeps the viewport when an oversized target spans both cursor guard edges', () => {
    expect(
      resolveTypewriterScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 250,
        targetBottom: 800,
        visibleArea: { topInset: 10, bottomInset: 20 },
        position: 0.5,
      }),
    ).toBeNull();
  });
});

describe('resolveKeepVisibleBottomPadding', () => {
  it('uses stable bottom padding for the cursor guard range', () => {
    expect(
      resolveKeepVisibleBottomPadding({
        visibleArea: { topInset: 0, bottomInset: 40 },
      }),
    ).toBe(100);
  });
});

describe('resolveScrollPastEndBottomPadding', () => {
  it('lets the document end reach the middle of the visible viewport', () => {
    expect(
      resolveScrollPastEndBottomPadding({
        clientHeight: 500,
        visibleArea: { topInset: 20, bottomInset: 40 },
        trailingBottomMargin: 20,
      }),
    ).toBe(240);
  });
});

describe('resolveTypewriterBottomPadding', () => {
  it('uses typewriter padding from viewport position and trailing margin', () => {
    expect(
      resolveTypewriterBottomPadding({
        clientHeight: 500,
        targetHeight: 20,
        visibleArea: { topInset: 0, bottomInset: 40 },
        position: 0.5,
        trailingBottomMargin: 20,
      }),
    ).toBe(240);
  });

  it('keeps the minimum bottom padding when typewriter space fits in the trailing margin', () => {
    expect(
      resolveTypewriterBottomPadding({
        clientHeight: 500,
        targetHeight: 20,
        position: 1,
        trailingBottomMargin: 20,
      }),
    ).toBe(48);
  });
});
