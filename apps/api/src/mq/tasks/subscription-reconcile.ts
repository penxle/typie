import { SubscriptionState } from '@typie/lib/enums';
import type dayjs from 'dayjs';

// 스토어에서 확인된 구독 상태. suspended = 보류·재청구·일시중지(권한 없음이나 복구 가능),
// expired = 일반 만료, revoked = 환불·철회, unknown = 조회 실패/모호(아무 것도 하지 않음).
export type ReconcileStoreState = 'active' | 'grace' | 'suspended' | 'expired' | 'revoked' | 'unknown';

export type ReconcileAction =
  { type: 'none' } | { type: 'recover'; expiresAt: dayjs.Dayjs } | { type: 'grace' } | { type: 'suspend' } | { type: 'expire' };

type ResolveReconcileActionParams = {
  storeState: ReconcileStoreState;
  storeExpiresAt: dayjs.Dayjs | null;
  currentState: SubscriptionState;
  currentExpiresAt: dayjs.Dayjs;
  now: dayjs.Dayjs;
};

// 스토어 상태와 (잠금 하에 재조회한) 현재 DB 상태를 받아 취할 동작을 결정하는 순수 함수.
// I/O 없이 결정만 담당해 상태×상태 매트릭스를 단위 테스트할 수 있게 한다.
export const resolveReconcileAction = ({
  storeState,
  storeExpiresAt,
  currentState,
  currentExpiresAt,
  now,
}: ResolveReconcileActionParams): ReconcileAction => {
  const isLive =
    currentState === SubscriptionState.ACTIVE ||
    currentState === SubscriptionState.WILL_EXPIRE ||
    currentState === SubscriptionState.IN_GRACE_PERIOD;

  const expired = currentExpiresAt.isBefore(now);

  // 유실된 갱신 복구. 스토어가 더 나중 만료일을 확인해 주면(=갱신 발생) 로컬 만료가 아직 지나기 전이라도 즉시 반영해,
  // 다음 일일 재조정까지 로컬의 옛 만료일에 걸려 접근이 끊기는 것을 막는다. 만료일이 실제로 뒤로 갈 때만 동작하므로
  // WILL_EXPIRE(해지 후 재개) 도 되살릴 수 있고, EXPIRED(환불/철회) 행은 isLive 에서 제외되어 되살리지 않는다.
  if (storeState === 'active' && storeExpiresAt && isLive && storeExpiresAt.isAfter(currentExpiresAt)) {
    return { type: 'recover', expiresAt: storeExpiresAt };
  }

  // 유예 접근 복원. 이미 유예/만료된 행은 대상이 아니다.
  if (storeState === 'grace' && (currentState === SubscriptionState.ACTIVE || currentState === SubscriptionState.WILL_EXPIRE) && expired) {
    return { type: 'grace' };
  }

  // 보류·재청구·일시중지: 접근은 중단(WILL_EXPIRE + 만료 경과 → 비권한)하되 재조정이 계속 감시해 복구 가능하게 둔다.
  if (
    storeState === 'suspended' &&
    (currentState === SubscriptionState.ACTIVE || currentState === SubscriptionState.IN_GRACE_PERIOD) &&
    expired
  ) {
    return { type: 'suspend' };
  }

  // expired: 스토어가 종료를 확인. 로컬 만료가 미래면 스토어가 결제 기간 전에 끝낸 것(철회·환불류)이므로 즉시 회수하고,
  //   로컬 만료가 이미 지났으면 일반 만료로 보되 갱신 직후 stale EXPIRED(스토어 API 지연) 대비 하루 여유를 둔다.
  // revoked: 환불·철회 — 남은 결제 기간과 무관하게 즉시 회수한다.
  if (
    isLive &&
    ((storeState === 'expired' && (currentExpiresAt.isAfter(now) || currentExpiresAt.isBefore(now.subtract(1, 'day')))) ||
      storeState === 'revoked')
  ) {
    return { type: 'expire' };
  }

  return { type: 'none' };
};
