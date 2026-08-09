import { resolveFloatingCenteredFallback } from '@typie/ui/actions';
import { describe, expect, it } from 'vitest';

const popover = { width: 280, height: 240 };
const viewport = { x: 8, y: 8, width: 984, height: 784 };

describe('resolveFloatingCenteredFallback', () => {
  it('keeps normal placement when only the space above fits', () => {
    expect(
      resolveFloatingCenteredFallback({
        reference: { x: 100, y: 600, width: 200, height: 40 },
        floating: popover,
        clippingRect: viewport,
        gap: 6,
      }),
    ).toBeNull();
  });

  it('keeps normal placement when only the space below fits', () => {
    expect(
      resolveFloatingCenteredFallback({
        reference: { x: 100, y: 80, width: 200, height: 40 },
        floating: popover,
        clippingRect: viewport,
        gap: 6,
      }),
    ).toBeNull();
  });

  it('does not override the existing below-first choice when both sides fit', () => {
    expect(
      resolveFloatingCenteredFallback({
        reference: { x: 100, y: 350, width: 200, height: 40 },
        floating: popover,
        clippingRect: viewport,
        gap: 6,
      }),
    ).toBeNull();
  });

  it('centers in the padded viewport when neither side fits', () => {
    expect(
      resolveFloatingCenteredFallback({
        reference: { x: 100, y: 200, width: 200, height: 400 },
        floating: popover,
        clippingRect: viewport,
        gap: 6,
      }),
    ).toEqual({ x: 360, y: 280 });
  });

  it('uses the provided padded viewport bounds for the centered fallback', () => {
    expect(
      resolveFloatingCenteredFallback({
        reference: { x: 200, y: 180, width: 200, height: 340 },
        floating: popover,
        clippingRect: { x: 108, y: 48, width: 784, height: 604 },
        gap: 6,
      }),
    ).toEqual({ x: 360, y: 230 });
  });
});
