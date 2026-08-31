import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import { isSubscriptionLive } from './entitlement.ts';
import type dayjs from 'dayjs';
import type { EntitlementJudgmentRow } from './entitlement.ts';
import type { NormalizedIap } from './iap-normalize.ts';

type TrackedIap = Extract<NormalizedIap, { kind: 'tracked' }>;

export type ConflictCandidate = {
  id: string;
  state: SubscriptionState;
  planAvailability: PlanAvailability;
  startsAt: dayjs.Dayjs;
  currentPeriodStartsAt: dayjs.Dayjs;
  currentPeriodEndsAt: dayjs.Dayjs;
};

// 확정 종료된 행의 복권은 스토어가 유효한 유료 기간을 확인해 준 경우로 제한한다. 토큰 교체·승인 의무는
// 이 게이트 밖이다 — 복권이 미뤄져도 새 계약은 추적 대상으로 남아야 다음 조회가 복구를 잡는다.
// 스토어 유예는 주기 종료를 전진시키지 않아 기간식만으로는 영원히 복권되지 않는다 — 유예 자체가 스토어가
// 확인해 준 권한이므로 복권 근거로 인정한다.
export const isRevivalGated = (canonicalState: SubscriptionState | undefined, normalized: TrackedIap, now: dayjs.Dayjs): boolean =>
  canonicalState === SubscriptionState.EXPIRED &&
  !normalized.periodEndsAt.isAfter(now) &&
  normalized.state !== SubscriptionState.IN_GRACE_PERIOD;

export const selectGooglePredecessor = <T extends { identifier: string }>(successorTokens: string[], rows: T[]): T | null =>
  successorTokens.flatMap((token) => rows.filter((row) => row.identifier === token))[0] ?? null;

// 승격 충돌은 명시 검사로 잡는다 — 유니크 위반은 경합의 최후 방어일 뿐이고, WILL_ACTIVATE 는 부분 유니크가
// 분리되어 있어 DB 가 잡지도 못한다. 반대로 이미 live 인 계보의 통상 갱신은 다른 IAP 계보와 대조하지 않는다 —
// 매 갱신이 알람이 되므로, 계보가 non-live 에서 live 로 바뀌는 전이에서만 대조한다.
export const findPromotionConflict = ({
  candidates,
  canonical,
  normalized,
  now,
}: {
  candidates: ConflictCandidate[];
  canonical: EntitlementJudgmentRow;
  normalized: NormalizedIap;
  now: dayjs.Dayjs;
}): ConflictCandidate | null => {
  if (normalized.kind !== 'tracked') {
    return null;
  }

  // ACTIVE 승격은 부분 유니크와 직결된다 — 유니크가 채널을 가리지 않으므로 계보의 liveness 와도, 후보의 채널과도 무관하게 검사한다.
  const activeConflict =
    normalized.state === SubscriptionState.ACTIVE
      ? candidates.find((row) => row.state === SubscriptionState.ACTIVE || row.state === SubscriptionState.WILL_ACTIVATE)
      : undefined;
  if (activeConflict) {
    return activeConflict;
  }

  if (isSubscriptionLive(canonical, now)) {
    return null;
  }

  const becomesLive = isSubscriptionLive(
    {
      state: normalized.state,
      planAvailability: PlanAvailability.IN_APP_PURCHASE,
      startsAt: normalized.periodStartsAt,
      currentPeriodStartsAt: normalized.periodStartsAt,
      currentPeriodEndsAt: normalized.periodEndsAt,
    },
    now,
  );
  if (!becomesLive) {
    return null;
  }

  return (
    candidates.find((row) =>
      row.planAvailability === PlanAvailability.IN_APP_PURCHASE
        ? isSubscriptionLive(row, now)
        : row.state === SubscriptionState.ACTIVE || row.state === SubscriptionState.WILL_ACTIVATE,
    ) ?? null
  );
};

export const RECONCILE_TERMINATED_WINDOW_DAYS = 90;

export const reconcileWindowStart = (now: dayjs.Dayjs): dayjs.Dayjs => now.subtract(RECONCILE_TERMINATED_WINDOW_DAYS, 'day');
