import { PlanAvailability } from '@typie/lib/enums';
import { isSubscriptionLive } from './entitlement.ts';
import type dayjs from 'dayjs';
import type { EntitlementJudgmentRow } from './entitlement.ts';

export type EnrollSubscriptionRow = EntitlementJudgmentRow;

export type EnrollAction = { kind: 'reject' } | { kind: 'immediate' } | { kind: 'schedule'; startsAt: dayjs.Dayjs };

// 유저의 비만료 구독 전 행을 받아 등록 방식을 결정하는 순수 함수. 단일 행 대표는 전환 직후
// ACTIVE+옛 트라이얼 공존 창에서 오판하므로 전 행 불변식으로 판정한다. 호출은 락 하 트랜잭션 안에서.
export const resolveEnrollAction = (rows: EnrollSubscriptionRow[], now: dayjs.Dayjs): EnrollAction => {
  const current = rows.filter((row) => isSubscriptionLive(row, now));

  if (current.some((row) => row.planAvailability !== PlanAvailability.TRIAL)) {
    return { kind: 'reject' };
  }

  const trial = current.find((row) => row.planAvailability === PlanAvailability.TRIAL);
  if (trial) {
    return { kind: 'schedule', startsAt: trial.currentPeriodEndsAt };
  }

  return { kind: 'immediate' };
};
