import { describe, expect, it } from 'vitest';
import { calculateImageContainerSize, calculateImageHeight, calculateImageWidth } from './image';

describe('image sizing', () => {
  it('uses the requested proportion without exceeding the original width', () => {
    expect(calculateImageWidth(800, 50, 1000)).toBe(400);
    expect(calculateImageWidth(800, 100, 320)).toBe(320);
    expect(calculateImageHeight(400, 1000, 500)).toBe(200);
  });

  it('keeps the same dimensions while uploading and after persistence', () => {
    const size = calculateImageContainerSize({
      boundsWidth: 800,
      proportion: 100,
      originalWidth: 320,
      originalHeight: 240,
    });

    expect(size).toEqual({ width: '320px', height: '240px' });
  });

  it('uses the full available width only before dimensions are known', () => {
    expect(
      calculateImageContainerSize({
        boundsWidth: 800,
        proportion: 100,
        originalWidth: 0,
        originalHeight: 0,
      }),
    ).toEqual({ width: '100%', height: undefined });
  });
});
