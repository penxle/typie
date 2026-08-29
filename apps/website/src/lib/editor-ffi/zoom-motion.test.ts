import { describe, expect, it } from 'vitest';
import { elasticDisplayZoom, ZOOM_ELASTIC_EXTENT_RATIO, ZoomMotion } from './zoom-motion';

const bounds = { min: 0.2, max: 2 };

describe('elasticDisplayZoom', () => {
  it('preserves the normal range and compresses overzoom without a nearby second wall', () => {
    expect(elasticDisplayZoom(1.5, bounds)).toBe(1.5);
    expect(elasticDisplayZoom(2, bounds)).toBe(2);

    const first = elasticDisplayZoom(2.1, bounds) ?? 0;
    const second = elasticDisplayZoom(2.2, bounds) ?? 0;
    expect(first).toBeGreaterThan(2);
    expect(first).toBeLessThan(2.1);
    expect(second).toBeGreaterThan(first);
    expect(second).toBeLessThan(2.2);
    expect(elasticDisplayZoom(100, bounds)).toBeLessThan(bounds.max * ZOOM_ELASTIC_EXTENT_RATIO);
  });
});

describe('ZoomMotion', () => {
  it('recovers direct overzoom to the normal bound', () => {
    const motion = createMotion(2.08);

    expect(motion.advance(1 / 60).displayZoom).toBeLessThan(2.08);
    expect(motion.advance(1).displayZoom).toBe(2);
  });

  it('recovers direct underzoom to the normal bound', () => {
    const motion = createMotion(0.18);

    expect(motion.advance(1 / 60).displayZoom).toBeGreaterThan(0.18);
    expect(motion.advance(1).displayZoom).toBe(0.2);
  });
});

function createMotion(displayZoom: number): ZoomMotion {
  return new ZoomMotion(displayZoom, bounds);
}
