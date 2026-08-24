import { describe, expect, test } from 'vitest';
import { createPrismIndicatorPath, samplePrismIndicatorPath } from './prism-indicator-path.ts';

describe('Prism indicator screen-space path', () => {
  test('follows one upward arc between its endpoints', () => {
    const path = createPrismIndicatorPath({ x: 200, y: 300 }, { x: 40, y: 80 });
    expect(samplePrismIndicatorPath(path, 0)).toEqual(path.p0);
    expect(samplePrismIndicatorPath(path, 1)).toEqual(path.p3);

    const interior = Array.from({ length: 15 }, (_, index) => samplePrismIndicatorPath(path, (index + 1) / 16));
    expect(
      interior.every((point) => {
        const lineProgress = (point.x - path.p0.x) / (path.p3.x - path.p0.x);
        const lineY = path.p0.y + (path.p3.y - path.p0.y) * lineProgress;
        return point.y < lineY;
      }),
    ).toBe(true);
  });

  test('rejects invalid endpoints', () => {
    expect(() => createPrismIndicatorPath({ x: NaN, y: 20 }, { x: 0, y: 0 })).toThrow(RangeError);
    expect(() => createPrismIndicatorPath({ x: 20, y: 20 }, { x: 20, y: 0 })).toThrow(RangeError);
    expect(() => createPrismIndicatorPath({ x: 20, y: 20 }, { x: 0, y: 20 })).toThrow(RangeError);
    expect(() => createPrismIndicatorPath({ x: 20, y: 20 }, { x: 30, y: 30 })).toThrow(RangeError);
  });
});
