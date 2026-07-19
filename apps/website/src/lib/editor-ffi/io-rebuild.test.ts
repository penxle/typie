import { describe, expect, it } from 'vitest';
import { createTrailingDebounce, IO_REBUILD_HEIGHT_THRESHOLD, shouldRebuildForResize } from './io-rebuild';

describe('io-rebuild: shouldRebuildForResize', () => {
  it('always rebuilds on the first build (no prior height)', () => {
    expect(shouldRebuildForResize(0, 800)).toBe(true);
    expect(shouldRebuildForResize(-1, 800)).toBe(true);
  });

  it('skips small deltas below the threshold (toolbar collapse ~10%)', () => {
    expect(shouldRebuildForResize(1000, 900)).toBe(false); // -10%
    expect(shouldRebuildForResize(1000, 1100)).toBe(false); // +10%
    expect(shouldRebuildForResize(1000, 1149)).toBe(false); // +14.9%
  });

  it('rebuilds at or above the threshold boundary (rotation / split-screen)', () => {
    expect(shouldRebuildForResize(1000, 1150)).toBe(true); // exactly +15%
    expect(shouldRebuildForResize(1000, 850)).toBe(true); // exactly -15%
    expect(shouldRebuildForResize(1000, 1400)).toBe(true); // +40%
  });

  it('honors a custom threshold', () => {
    expect(shouldRebuildForResize(1000, 1050, 0.02)).toBe(true); // +5% >= 2%
    expect(shouldRebuildForResize(1000, 1050, 0.1)).toBe(false); // +5% < 10%
  });

  it('default threshold is 0.15', () => {
    expect(IO_REBUILD_HEIGHT_THRESHOLD).toBe(0.15);
  });
});

describe('io-rebuild: createTrailingDebounce', () => {
  type Timer = { fn: () => void; ms: number; cancelled: boolean };
  const fakeSchedule = () => {
    const timers: Timer[] = [];
    const schedule = (fn: () => void, ms: number) => {
      const timer: Timer = { fn, ms, cancelled: false };
      timers.push(timer);
      return () => {
        timer.cancelled = true;
      };
    };
    const active = () => timers.filter((t) => !t.cancelled);
    // 실제 setTimeout처럼 발화한 타이머는 소진(재발화 불가)된다.
    const fire = (timer: Timer) => {
      timer.cancelled = true;
      timer.fn();
    };
    return { timers, schedule, active, fire };
  };

  it('collapses rapid calls into a single trailing execution', () => {
    const { schedule, active, fire } = fakeSchedule();
    const debounce = createTrailingDebounce(schedule, 300);
    let ran = '';
    debounce.call(() => (ran += 'a'));
    debounce.call(() => (ran += 'b'));
    debounce.call(() => (ran += 'c'));

    // 앞선 두 예약은 취소되고 마지막 하나만 살아있다.
    expect(active()).toHaveLength(1);
    expect(active()[0].ms).toBe(300);

    fire(active()[0]);
    expect(ran).toBe('c'); // 마지막 콜백만 1회 실행
  });

  it('cancel() clears a pending execution', () => {
    const { schedule, active } = fakeSchedule();
    const debounce = createTrailingDebounce(schedule, 300);
    let ran = false;
    debounce.call(() => (ran = true));
    debounce.cancel();
    expect(active()).toHaveLength(0);
    expect(ran).toBe(false);
  });

  it('re-arms after a fired execution', () => {
    const { schedule, active, fire } = fakeSchedule();
    const debounce = createTrailingDebounce(schedule, 300);
    let count = 0;
    debounce.call(() => (count += 1));
    fire(active()[0]);
    debounce.call(() => (count += 1));
    expect(active()).toHaveLength(1);
    fire(active()[0]);
    expect(count).toBe(2);
  });
});
