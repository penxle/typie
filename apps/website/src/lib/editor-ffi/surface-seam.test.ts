import { describe, expect, it } from 'vitest';
import { seamSamplePoints } from './surface-seam';

describe('seamSamplePoints', () => {
  it('returns no points for a single-tile page', () => {
    expect(seamSamplePoints([0], 200, 400)).toEqual([]);
    expect(seamSamplePoints([], 200, 400)).toEqual([]);
  });

  it('samples above-and-at each interior tile boundary across columns', () => {
    const points = seamSamplePoints([0, 100, 200], 200, 300, 2);
    // Two interior boundaries (100, 200), each sampled at rows (y0-2, y0) over 3 columns.
    const rows = new Set(points.map(([, y]) => y));
    expect(rows).toEqual(new Set([98, 100, 198, 200]));
    // Columns: 8, floor(200/2)-1 = 99, 200-8-2 = 190.
    const cols = new Set(points.map(([x]) => x));
    expect(cols).toEqual(new Set([8, 99, 190]));
    expect(points.length).toBe(4 * 3);
  });

  it('skips the top boundary (y0 <= 0) and boundaries at/below the bottom edge', () => {
    // A boundary exactly at height is not interior.
    expect(seamSamplePoints([0, 300], 100, 300, 2)).toEqual([]);
  });

  it('clamps sample points inside [0, size - block] and dedupes overlaps', () => {
    // Narrow page: all three columns collapse toward 0, so per row they dedupe.
    const points = seamSamplePoints([0, 5], 6, 10, 2);
    for (const [x, y] of points) {
      expect(x).toBeGreaterThanOrEqual(0);
      expect(x).toBeLessThanOrEqual(6 - 2);
      expect(y).toBeGreaterThanOrEqual(0);
      expect(y).toBeLessThanOrEqual(10 - 2);
    }
    // No duplicate coordinates survive.
    const keys = points.map(([x, y]) => `${x},${y}`);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('returns nothing when the page is smaller than a block', () => {
    expect(seamSamplePoints([0, 1], 1, 1, 2)).toEqual([]);
  });
});
