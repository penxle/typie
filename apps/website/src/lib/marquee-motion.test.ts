import { advanceMarqueeMotion } from '@typie/ui/utils';
import { describe, expect, it } from 'vitest';

const motionAt = (maximum: number, elapsed: number) => {
  let state = { position: 0, velocity: 0 };

  for (let time = 0; time < elapsed; time++) {
    state = advanceMarqueeMotion({ ...state, maximum, elapsed: 1 });
  }

  return state;
};

describe('marquee motion', () => {
  it('eases around one fixed spatial rate for a stable endpoint', () => {
    expect(motionAt(48, 100).position).toBeCloseTo(1.2, 1);
    expect(motionAt(48, 200)).toEqual({ position: expect.closeTo(4.8, 1), velocity: expect.closeTo(0.048, 4) });
    expect(motionAt(48, 300).position - motionAt(48, 200).position).toBeCloseTo(4.8, 1);
    expect(motionAt(48, 800).position - motionAt(48, 700).position).toBeCloseTo(4.8, 1);
    expect(motionAt(48, 1100).position).toBeCloseTo(46.8, 1);
    expect(motionAt(48, 1200)).toEqual({ position: 48, velocity: 0 });
  });

  it('uses symmetric ramps without exceeding the fixed rate for a short overflow', () => {
    expect(motionAt(8, 91).position).toBeCloseTo(1, 1);
    expect(motionAt(8, 183).position).toBeCloseTo(4, 1);
    expect(motionAt(8, 274).position).toBeCloseTo(7, 1);
    expect(motionAt(8, 366)).toEqual({ position: 8, velocity: 0 });
  });

  it('keeps position and velocity continuous when the endpoint grows', () => {
    let state = { position: 0, velocity: 0 };

    for (let elapsed = 0; elapsed < 600; elapsed += 10) {
      state = advanceMarqueeMotion({ ...state, maximum: 48, elapsed: 10 });
    }

    const beforeResize = state;
    state = advanceMarqueeMotion({ ...state, maximum: 72, elapsed: 16 });

    expect(state.position).toBeGreaterThan(beforeResize.position);
    expect(state.velocity).toBeCloseTo(beforeResize.velocity, 4);
    expect(state.velocity).toBeLessThanOrEqual(0.048);
  });

  it('clamps only to a nearer endpoint and resumes from a settled position when it grows again', () => {
    const clamped = advanceMarqueeMotion({ position: 30, velocity: 0.048, maximum: 20, elapsed: 16 });
    expect(clamped).toEqual({ position: 20, velocity: 0 });

    const resumed = advanceMarqueeMotion({ ...clamped, maximum: 35, elapsed: 16 });
    expect(resumed.position).toBeGreaterThan(20);
    expect(resumed.velocity).toBeGreaterThan(0);
    expect(resumed.velocity).toBeLessThanOrEqual(0.048);
  });

  it('does not move for a non-positive endpoint', () => {
    expect(advanceMarqueeMotion({ position: 0, velocity: 0, maximum: 0, elapsed: 100 })).toEqual({ position: 0, velocity: 0 });
  });
});
