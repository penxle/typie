import { SUBSCRIPTION_GRACE_DAYS } from '@typie/lib/const';
import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import type dayjs from 'dayjs';

// 권한·liveness 판정에 필요한 최소 형태 — 판정 함수는 이 형태만 요구해 크론·게이트가 id·createdAt 없이도 먹일 수 있다.
export type EntitlementJudgmentRow = {
  state: SubscriptionState;
  planAvailability: PlanAvailability;
  startsAt: dayjs.Dayjs;
  currentPeriodStartsAt: dayjs.Dayjs;
  currentPeriodEndsAt: dayjs.Dayjs;
};

export type EntitlementSubscriptionRow = EntitlementJudgmentRow & {
  id: string;
  createdAt: dayjs.Dayjs;
};

const IAP_GRACE_BACKSTOP_DAYS = 31;
const SHIM_FAIL_OPEN_HOURS = 48;

const earlierOf = (a: dayjs.Dayjs, b: dayjs.Dayjs): dayjs.Dayjs => (a.isBefore(b) ? a : b);
const laterOf = (a: dayjs.Dayjs, b: dayjs.Dayjs): dayjs.Dayjs => (a.isAfter(b) ? a : b);

// IAP는 스토어 유예 마감을 저장하지 않으므로 주기 종료 + 고정 백스톱이다. 그 외 채널은
// 미결제 주기의 시작(주기 컬럼 중 now 이하의 최댓값)이 유예 진입 시점이고, 거기서 7일이다.
// 주기 컬럼이 둘 다 미래인 경우(전제 밖)는 fail-open 방향으로 시작 컬럼을 기준 삼는다.
export const deriveGraceDeadline = (row: EntitlementJudgmentRow, now: dayjs.Dayjs): dayjs.Dayjs => {
  if (row.planAvailability === PlanAvailability.IN_APP_PURCHASE) {
    return row.currentPeriodEndsAt.add(IAP_GRACE_BACKSTOP_DAYS, 'day');
  }

  const elapsed = [row.currentPeriodStartsAt, row.currentPeriodEndsAt].filter((at) => !at.isAfter(now));
  const base = elapsed.length > 0 ? elapsed.reduce(laterOf) : row.currentPeriodStartsAt;

  return base.add(SUBSCRIPTION_GRACE_DAYS, 'day');
};

export const isSubscriptionEntitled = (row: EntitlementJudgmentRow, now: dayjs.Dayjs): boolean => {
  switch (row.state) {
    case SubscriptionState.ACTIVE: {
      return true;
    }
    case SubscriptionState.WILL_EXPIRE: {
      return row.currentPeriodEndsAt.isAfter(now);
    }
    case SubscriptionState.IN_GRACE_PERIOD: {
      return deriveGraceDeadline(row, now).isAfter(now);
    }
    case SubscriptionState.EXPIRED: {
      return false;
    }
    case SubscriptionState.WILL_ACTIVATE: {
      return !row.startsAt.isAfter(now);
    }
  }
};

// 게이트 liveness — "이 행이 지금 등록·전이를 막는 살아 있는 구독인가"의 단일 판정. 권한식과 같은 식을 쓴다:
// 저장 상태만 보면 확정 잡 지연(해지 매분, 유예 소진) 동안 잠긴 유저의 재가입이 차단된다. WILL_ACTIVATE 는
// 시각과 무관하게 제외한다 — 예약의 대체·부활 판정은 예약 기계(retireReservation·전환 잡 CAS)의 소관이다.
export const isSubscriptionLive = (row: EntitlementJudgmentRow, now: dayjs.Dayjs): boolean =>
  row.state !== SubscriptionState.WILL_ACTIVATE && isSubscriptionEntitled(row, now);

const deriveEffectiveDeadline = (row: EntitlementJudgmentRow, now: dayjs.Dayjs): dayjs.Dayjs => {
  if (row.state === SubscriptionState.IN_GRACE_PERIOD) {
    return deriveGraceDeadline(row, now);
  }

  return row.currentPeriodEndsAt;
};

export const resolveUserEntitlement = (
  rows: EntitlementSubscriptionRow[],
  now: dayjs.Dayjs,
): { entitled: boolean; entitledUntil: dayjs.Dayjs | null } => {
  const entitledRows = rows.filter((row) => isSubscriptionEntitled(row, now));
  if (entitledRows.length === 0) {
    return { entitled: false, entitledUntil: null };
  }

  // ACTIVE·시작 경과 WILL_ACTIVATE는 시각으로 종결되지 않으므로 entitledUntil이 없다.
  const hasIndefiniteRow = entitledRows.some(
    (row) => row.state === SubscriptionState.ACTIVE || row.state === SubscriptionState.WILL_ACTIVATE,
  );
  if (hasIndefiniteRow) {
    return { entitled: true, entitledUntil: null };
  }

  const entitledUntil = entitledRows.map((row) => deriveEffectiveDeadline(row, now)).reduce(laterOf);

  return { entitled: true, entitledUntil };
};

export const selectRepresentativeSubscription = <T extends EntitlementSubscriptionRow>(rows: T[], now: dayjs.Dayjs): T | null => {
  // 시작 경과 WILL_ACTIVATE도 후보다 — 전환 지연 창에서 유일한 entitled 행이 예약 행일 수 있다.
  const candidates = rows.filter(
    (row) =>
      row.state === SubscriptionState.ACTIVE ||
      row.state === SubscriptionState.WILL_EXPIRE ||
      row.state === SubscriptionState.IN_GRACE_PERIOD ||
      (row.state === SubscriptionState.WILL_ACTIVATE && !row.startsAt.isAfter(now)),
  );
  if (candidates.length === 0) {
    return null;
  }

  const rank = (row: EntitlementSubscriptionRow): [number, number] => [
    isSubscriptionEntitled(row, now) ? 0 : 1,
    row.state === SubscriptionState.ACTIVE ? 0 : 1,
  ];

  return candidates.reduce((best, row) => {
    const [bestEntitledRank, bestActiveRank] = rank(best);
    const [rowEntitledRank, rowActiveRank] = rank(row);

    if (rowEntitledRank !== bestEntitledRank) {
      return rowEntitledRank < bestEntitledRank ? row : best;
    }
    if (rowActiveRank !== bestActiveRank) {
      return rowActiveRank < bestActiveRank ? row : best;
    }
    return row.createdAt.isAfter(best.createdAt) ? row : best;
  });
};

// 구 앱이 IN_GRACE_PERIOD에서 스스로 7일(빌링키)/31일(IAP)을 더하므로 여기서는 역산값을 낸다.
export const deriveExpiresAtShim = (row: EntitlementJudgmentRow, now: dayjs.Dayjs): dayjs.Dayjs => {
  switch (row.state) {
    case SubscriptionState.ACTIVE: {
      return row.planAvailability === PlanAvailability.BILLING_KEY
        ? row.currentPeriodEndsAt.add(SHIM_FAIL_OPEN_HOURS, 'hour')
        : row.currentPeriodEndsAt;
    }
    case SubscriptionState.WILL_EXPIRE: {
      return row.currentPeriodEndsAt;
    }
    case SubscriptionState.IN_GRACE_PERIOD: {
      return row.planAvailability === PlanAvailability.IN_APP_PURCHASE
        ? row.currentPeriodEndsAt
        : deriveGraceDeadline(row, now).subtract(SUBSCRIPTION_GRACE_DAYS, 'day');
    }
    case SubscriptionState.EXPIRED: {
      return earlierOf(row.currentPeriodEndsAt, now);
    }
    case SubscriptionState.WILL_ACTIVATE: {
      return row.startsAt.isAfter(now) ? earlierOf(row.startsAt, now) : row.startsAt.add(SHIM_FAIL_OPEN_HOURS, 'hour');
    }
  }
};
