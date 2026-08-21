import { describe, expect, it } from 'vitest';
import { backoffDelay } from './backoff.ts';

const delays = [1000, 3000, 10_000];

describe('backoffDelay', () => {
  it('첫 실패는 첫 간격을 준다', () => {
    expect(backoffDelay(delays, 1)).toBe(1000);
  });

  it('실패 횟수만큼 뒤의 간격을 준다', () => {
    expect(backoffDelay(delays, 3)).toBe(10_000);
  });

  it('간격을 다 쓰면 null', () => {
    expect(backoffDelay(delays, 4)).toBeNull();
    expect(backoffDelay([], 1)).toBeNull();
  });
});
