import { describe, expect, it } from 'vitest';
import { SmoothScrollMotion } from './smooth-scroll-motion';

describe('SmoothScrollMotion', () => {
  it('translates the coordinate system without changing velocity or remaining distance', () => {
    const motion = SmoothScrollMotion.start({ position: 100, target: 500, viewportHeight: 400 });
    motion.advance(1 / 60);
    const before = motion.snapshot();

    motion.translate(700);

    const after = motion.snapshot();
    expect(after.position).toBeCloseTo(before.position + 700, 10);
    expect(after.target).toBeCloseTo(before.target + 700, 10);
    expect(after.velocity).toBeCloseTo(before.velocity, 10);
    expect(after.target - after.position).toBeCloseTo(before.target - before.position, 10);
  });

  it('produces the same state for equal elapsed time across frame rates', () => {
    const at60Hz = SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 });
    const at120Hz = SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 });

    repeat(30, () => at60Hz.advance(1 / 60));
    repeat(60, () => at120Hz.advance(1 / 120));

    expect(at60Hz.snapshot().position).toBeCloseTo(at120Hz.snapshot().position, 8);
    expect(at60Hz.snapshot().velocity).toBeCloseTo(at120Hz.snapshot().velocity, 8);
  });

  it('matches the cross-host retarget and translation vector', () => {
    const motion = SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 });
    motion.advance(0.05);
    motion.advance(0.05);
    motion.translate(700);
    motion.retarget(2100, 400);
    motion.advance(0.05);

    expect(motion.snapshot().position).toBeCloseTo(1809.7452631440876, 8);
    expect(motion.snapshot().velocity).toBeCloseTo(4556.356253653357, 8);
    expect(motion.snapshot().target).toBe(2100);
  });

  it('takes longer and reaches a higher speed for a farther destination', () => {
    const short = runToCompletion(SmoothScrollMotion.start({ position: 0, target: 100, viewportHeight: 400 }));
    const long = runToCompletion(SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 }));

    expect(long.elapsed).toBeGreaterThan(short.elapsed);
    expect(long.elapsed).toBeLessThan(short.elapsed * 4);
    expect(long.peakVelocity).toBeGreaterThan(short.peakVelocity);
  });

  it('retargets without changing the current position or velocity', () => {
    const motion = SmoothScrollMotion.start({ position: 0, target: 800, viewportHeight: 400 });
    repeat(8, () => motion.advance(1 / 60));
    const before = motion.snapshot();

    motion.retarget(1200, 400);

    expect(motion.snapshot().position).toBe(before.position);
    expect(motion.snapshot().velocity).toBe(before.velocity);
    expect(motion.snapshot().target).toBe(1200);
  });

  it('finishes at a target within the position threshold even with remaining velocity', () => {
    const motion = SmoothScrollMotion.start({ position: 0, target: 800, viewportHeight: 400 });
    repeat(8, () => motion.advance(1 / 60));
    const current = motion.snapshot().position;

    motion.retarget(current + 0.25, 400);

    expect(motion.finished).toBe(true);
    expect(motion.snapshot()).toEqual({ position: current + 0.25, velocity: 0, target: current + 0.25 });
  });

  it('preserves velocity while preventing a high-speed closer retarget from overshooting', () => {
    const motion = SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 });
    repeat(12, () => motion.advance(1 / 60));
    const before = motion.snapshot();
    const closerTarget = before.position + 80;

    motion.retarget(closerTarget, 400);
    expect(motion.snapshot().velocity).toBe(before.velocity);

    while (!motion.finished) {
      expect(motion.advance(1 / 120).position).toBeLessThanOrEqual(closerTarget);
    }
    expect(motion.snapshot()).toEqual({ position: closerTarget, velocity: 0, target: closerTarget });
  });

  it('synchronizes host-clamped bounds and removes only outward velocity', () => {
    const motion = SmoothScrollMotion.start({ position: 0, target: 1600, viewportHeight: 400 });
    repeat(6, () => motion.advance(1 / 60));

    motion.synchronizeBounds(600, 600, 400);

    expect(motion.snapshot()).toEqual({ position: 600, velocity: 0, target: 600 });
    expect(motion.finished).toBe(true);
  });

  it('catches up to the full elapsed time after a delayed frame', () => {
    const delayed = SmoothScrollMotion.start({ position: 0, target: 800, viewportHeight: 400 });
    const uninterrupted = SmoothScrollMotion.start({ position: 0, target: 800, viewportHeight: 400 });

    delayed.advance(0.25);
    repeat(15, () => uninterrupted.advance(1 / 60));

    expect(delayed.snapshot().position).toBeCloseTo(uninterrupted.snapshot().position, 8);
    expect(delayed.snapshot().velocity).toBeCloseTo(uninterrupted.snapshot().velocity, 8);
  });
});

function repeat(count: number, block: () => void): void {
  for (let index = 0; index < count; index += 1) block();
}

function runToCompletion(motion: SmoothScrollMotion): { elapsed: number; peakVelocity: number } {
  let elapsed = 0;
  let peakVelocity = 0;
  while (!motion.finished && elapsed < 5) {
    const state = motion.advance(1 / 120);
    elapsed += 1 / 120;
    peakVelocity = Math.max(peakVelocity, Math.abs(state.velocity));
  }
  expect(motion.finished).toBe(true);
  return { elapsed, peakVelocity };
}
