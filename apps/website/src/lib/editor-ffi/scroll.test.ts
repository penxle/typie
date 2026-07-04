import { describe, expect, it } from 'vitest';
import {
  resolveKeepVisibleBottomPadding,
  resolveNearestScrollTop,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';

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

  it('aligns an oversized target top to the guarded visible area', () => {
    expect(
      resolveNearestScrollTop({
        scrollTop: 300,
        clientHeight: 400,
        scrollHeight: 2000,
        targetTop: 1000,
        targetBottom: 1500,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toBe(930);
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

describe('resolveKeepVisibleBottomPadding', () => {
  it('uses stable bottom padding for the cursor guard range', () => {
    expect(
      resolveKeepVisibleBottomPadding({
        visibleArea: { topInset: 0, bottomInset: 40 },
      }),
    ).toBe(100);
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
