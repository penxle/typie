import { describe, expect, it } from 'vitest';
import { resolveNearestScrollTop, resolveTypewriterBottomPadding, resolveTypewriterScrollTop } from './scroll';

describe('resolveNearestScrollTop', () => {
  it('keeps the target inside the guarded visible area with insets', () => {
    expect(
      resolveNearestScrollTop({
        scrollTop: 100,
        clientHeight: 400,
        scrollHeight: 1000,
        targetTop: 430,
        targetBottom: 450,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(130);

    expect(
      resolveNearestScrollTop({
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
      resolveNearestScrollTop({
        scrollTop: 100,
        clientHeight: 400,
        scrollHeight: 1000,
        targetTop: 220,
        targetBottom: 240,
      }),
    ).toBeNull();
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
});

describe('resolveTypewriterBottomPadding', () => {
  it('grows when content bottom is too close to the cursor line', () => {
    expect(
      resolveTypewriterBottomPadding({
        clientHeight: 500,
        targetHeight: 20,
        distanceToContentBottom: 80,
        visibleArea: { topInset: 0, bottomInset: 40 },
        position: 0.5,
      }),
    ).toBe(200);
  });

  it('keeps the minimum bottom padding when existing content space is enough', () => {
    expect(
      resolveTypewriterBottomPadding({
        clientHeight: 500,
        targetHeight: 20,
        distanceToContentBottom: 500,
        position: 0.5,
      }),
    ).toBe(48);
  });
});
