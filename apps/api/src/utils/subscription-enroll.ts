import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import type dayjs from 'dayjs';

export type EnrollSubscriptionRow = {
  state: SubscriptionState;
  planAvailability: PlanAvailability;
  expiresAt: dayjs.Dayjs;
};

export type EnrollAction = { kind: 'reject' } | { kind: 'immediate' } | { kind: 'schedule'; startsAt: dayjs.Dayjs };

// 유저의 비만료 구독 전 행을 받아 등록 방식을 결정하는 순수 함수. 단일 행 대표는 전환 직후
// ACTIVE+옛 트라이얼 공존 창에서 오판하므로 전 행 불변식으로 판정한다. 호출은 락 하 트랜잭션 안에서.
export const resolveEnrollAction = (rows: EnrollSubscriptionRow[], now: dayjs.Dayjs): EnrollAction => {
  // 시간상 만료된 WILL_EXPIRE 는 권한 판정과 동일하게 비활성으로 취급한다 — 상태만 보면
  // 해지 확정 잡 지연 동안 재가입이 차단된다.
  const current = rows.filter(
    (row) =>
      row.state !== SubscriptionState.EXPIRED &&
      row.state !== SubscriptionState.WILL_ACTIVATE &&
      !(row.state === SubscriptionState.WILL_EXPIRE && !row.expiresAt.isAfter(now)),
  );

  if (current.some((row) => row.planAvailability !== PlanAvailability.TRIAL)) {
    return { kind: 'reject' };
  }

  const trial = current.find((row) => row.planAvailability === PlanAvailability.TRIAL);
  if (trial) {
    return { kind: 'schedule', startsAt: trial.expiresAt };
  }

  return { kind: 'immediate' };
};
