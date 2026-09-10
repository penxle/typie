import { describe, expect, it } from 'vitest';
import { calculateImageContainerSize, calculateImageHeight, calculateImageWidth } from './image';

describe('image sizing', () => {
  it('uses the requested proportion without exceeding the original width', () => {
    expect(calculateImageWidth(800, 50, 1000, 500)).toBe(400);
    expect(calculateImageWidth(800, 100, 320, 240)).toBe(320);
    expect(calculateImageWidth(800, 50, 320, 240)).toBe(160);
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

  it('fits a tall image inside a full page without changing its aspect ratio', () => {
    expect(
      calculateImageContainerSize({
        boundsWidth: 600,
        proportion: 100,
        originalWidth: 600,
        originalHeight: 3000,
        maxHeight: 800,
      }),
    ).toEqual({ width: '160px', height: '800px' });
    expect(calculateImageWidth(600, 20, 600, 3000, 800)).toBe(32);
    expect(calculateImageWidth(600, 100, 600, 3000)).toBe(600);
    expect(calculateImageWidth(600, 100, 600, 12_000, 800)).toBe(40);
    expect(calculateImageWidth(600, 10, 600, 12_000, 800)).toBe(4);
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
