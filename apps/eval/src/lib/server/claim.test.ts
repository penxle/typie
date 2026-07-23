import { describe, expect, it } from 'vitest';
import { reserveAllowance } from './claim.ts';

describe('reserveAllowance', () => {
  it('뒤처진 다른 평가자들의 부족분만큼 남은 작업에서 떼어 둔다', () => {
    // 필요 40, 8명 → 최소 몫 5. 타인 7명 중 2명이 0건·3건 → 부족분 5+2=7.
    const allowance = reserveAllowance({
      requiredTotal: 40,
      effectiveDone: 20,
      participants: 8,
      othersConfirmed: [5, 6, 5, 8, 0, 3, 5],
    });
    expect(allowance).toBe(40 - 20 - 7);
  });

  it('전원이 최소 몫을 채웠으면 남은 작업 전부를 허용한다', () => {
    const allowance = reserveAllowance({ requiredTotal: 40, effectiveDone: 30, participants: 4, othersConfirmed: [10, 10, 10] });
    expect(allowance).toBe(10);
  });

  it('남은 작업이 예약분 이하이면 0', () => {
    const allowance = reserveAllowance({ requiredTotal: 40, effectiveDone: 36, participants: 8, othersConfirmed: [8, 8, 8, 8, 8, 8, 0] });
    expect(allowance).toBe(0);
  });

  it('참여자가 1명 이하이면 예약이 없다', () => {
    expect(reserveAllowance({ requiredTotal: 40, effectiveDone: 0, participants: 1, othersConfirmed: [] })).toBe(Infinity);
  });
});
