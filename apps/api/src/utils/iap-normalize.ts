import { AutoRenewStatus, InAppOwnershipType, Status } from '@apple/app-store-server-library';
import { PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { isSubscriptionLive } from './entitlement.ts';
import type { JWSRenewalInfoDecodedPayload, JWSTransactionDecodedPayload } from '@apple/app-store-server-library';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';
import type { InAppPurchaseStore } from '@typie/lib/enums';
import type { EntitlementJudgmentRow, EntitlementSubscriptionRow } from './entitlement.ts';
import type { OpsAlertId } from './ops-alert.ts';

export type IapRenewalIntent = 'ON' | 'OFF' | 'unknown';

export type IapPriorPeriod = {
  state: SubscriptionState;
  currentPeriodStartsAt: dayjs.Dayjs;
  currentPeriodEndsAt: dayjs.Dayjs;
} | null;

export type NormalizedIap =
  | {
      kind: 'tracked';
      state: typeof SubscriptionState.ACTIVE | typeof SubscriptionState.WILL_EXPIRE | typeof SubscriptionState.IN_GRACE_PERIOD;
      planKey: string;
      periodStartsAt: dayjs.Dayjs;
      periodEndsAt: dayjs.Dayjs;
      intent: IapRenewalIntent;
      successorTokens: string[];
      acknowledgePending: boolean;
      productId: string | null;
    }
  | { kind: 'expired'; observed: { periodStartsAt: dayjs.Dayjs; periodEndsAt: dayjs.Dayjs; planKey: string } | null }
  | { kind: 'defer'; reason: string }
  | { kind: 'untracked'; reason: 'pending' | 'pending-canceled' }
  | { kind: 'unknown'; reason: string };

// 약정 상품(commitment·installment)과 미지 replacement mode는 현 모델(Plan.interval 단일 주기)로 표현되지
// 않는다 — 다른 unknown 사유(항목 미발견 등)와 달리 일일 재조정의 침묵 재시도로 자연 해소되지 않으므로
// 사람이 상품 구성을 확인해야 한다. 호출부(등록·웹훅·재조정)가 각자 opsAlertOnce 로 발화한다 — 이 함수는
// 사유→알람 ID 매핑만 담당해 세 호출부가 같은 사유 집합에서 갈라지지 않게 한다.
const UNSUPPORTED_STORE_PAYLOAD_REASONS = new Set([
  'apple-commitment-observed',
  'google-installment-observed',
  'google-replacement-mode-unrecognized',
]);

export const mapUnsupportedStorePayloadReason = (reason: string): OpsAlertId | null =>
  UNSUPPORTED_STORE_PAYLOAD_REASONS.has(reason) ? 'iap-unsupported-store-payload' : null;

export type AppleStatusItem = {
  status: number;
  outerOriginalTransactionId: string | null;
  transaction: JWSTransactionDecodedPayload | null;
  renewalInfo: JWSRenewalInfoDecodedPayload | null;
  subscriptionGroupIdentifier: string | null;
};

type GoogleLineItem = androidpublisher_v3.Schema$SubscriptionPurchaseLineItem;

const GOOGLE_STATE = {
  ACTIVE: 'SUBSCRIPTION_STATE_ACTIVE',
  CANCELED: 'SUBSCRIPTION_STATE_CANCELED',
  IN_GRACE_PERIOD: 'SUBSCRIPTION_STATE_IN_GRACE_PERIOD',
  ON_HOLD: 'SUBSCRIPTION_STATE_ON_HOLD',
  PAUSED: 'SUBSCRIPTION_STATE_PAUSED',
  EXPIRED: 'SUBSCRIPTION_STATE_EXPIRED',
  PENDING: 'SUBSCRIPTION_STATE_PENDING',
  PENDING_PURCHASE_CANCELED: 'SUBSCRIPTION_STATE_PENDING_PURCHASE_CANCELED',
} as const;

const ACKNOWLEDGEMENT_STATE_PENDING = 'ACKNOWLEDGEMENT_STATE_PENDING';

const NEW_SERVICE_PERIOD_MODES = new Set(['WITH_TIME_PRORATION', 'CHARGE_FULL_PRICE']);
const PLAN_ONLY_MODES = new Set(['CHARGE_PRORATED_PRICE', 'WITHOUT_PRORATION']);
const DEFERRED_MODE = 'DEFERRED';

const STORE_LAG_BUFFER_DAYS = 1;

const laterOf = (a: dayjs.Dayjs, b: dayjs.Dayjs): dayjs.Dayjs => (a.isAfter(b) ? a : b);

const addInterval = (at: dayjs.Dayjs, interval: PlanInterval): dayjs.Dayjs =>
  at.kst().add(1, interval === PlanInterval.YEARLY ? 'year' : 'month');

const subtractInterval = (at: dayjs.Dayjs, interval: PlanInterval): dayjs.Dayjs =>
  at.kst().subtract(1, interval === PlanInterval.YEARLY ? 'year' : 'month');

const parseEpoch = (value: number | undefined): dayjs.Dayjs | null => (value === undefined ? null : dayjs(value));

const parseTimestamp = (value: string | null | undefined): dayjs.Dayjs | null => (value ? dayjs(value) : null);

// 스토어가 종결을 알렸어도 로컬 주기가 막 끝난 직후면 갱신 반영 지연과 구분되지 않는다. 다음 조회가 백스톱이다.
// 경계는 현행 재조정과 동일하게 포함이다(마감 정각의 조회가 회수 쪽으로 기울지 않는다).
const isWithinStoreLagBuffer = (prior: IapPriorPeriod, now: dayjs.Dayjs): boolean =>
  !!prior && !prior.currentPeriodEndsAt.isAfter(now) && !prior.currentPeriodEndsAt.add(STORE_LAG_BUFFER_DAYS, 'day').isBefore(now);

export const selectAppleStatusItem = (
  items: AppleStatusItem[],
  requestedOriginalTransactionId: string,
): { kind: 'selected'; item: AppleStatusItem } | { kind: 'unknown'; reason: string } => {
  const related = items.filter(
    (item) =>
      item.transaction?.originalTransactionId === requestedOriginalTransactionId ||
      item.outerOriginalTransactionId === requestedOriginalTransactionId,
  );

  const mismatched = related.some(
    (item) =>
      item.outerOriginalTransactionId !== null &&
      item.transaction?.originalTransactionId !== undefined &&
      item.outerOriginalTransactionId !== item.transaction.originalTransactionId,
  );
  if (mismatched) {
    return { kind: 'unknown', reason: 'apple-transaction-id-mismatch' };
  }

  const candidates = items.filter((item) => item.transaction?.originalTransactionId === requestedOriginalTransactionId);
  if (candidates.length === 0) {
    return { kind: 'unknown', reason: 'apple-item-not-found' };
  }
  if (candidates.length === 1) {
    return { kind: 'selected', item: candidates[0] };
  }

  if (candidates.some((item) => item.transaction?.signedDate === undefined)) {
    return { kind: 'unknown', reason: 'apple-signed-date-missing' };
  }

  const latest = candidates.reduce((best, item) =>
    (item.transaction?.signedDate ?? 0) > (best.transaction?.signedDate ?? 0) ? item : best,
  );
  const tied = candidates.filter((item) => item.transaction?.signedDate === latest.transaction?.signedDate);
  if (tied.length > 1) {
    return { kind: 'unknown', reason: 'apple-signed-date-tie' };
  }

  return { kind: 'selected', item: latest };
};

const resolveAppleIntent = (renewalInfo: JWSRenewalInfoDecodedPayload | null): IapRenewalIntent => {
  if (renewalInfo?.autoRenewStatus === AutoRenewStatus.ON) {
    return 'ON';
  }
  if (renewalInfo?.autoRenewStatus === AutoRenewStatus.OFF) {
    return 'OFF';
  }

  return 'unknown';
};

const observeApple = (transaction: JWSTransactionDecodedPayload | null) => {
  const periodStartsAt = parseEpoch(transaction?.purchaseDate);
  const periodEndsAt = parseEpoch(transaction?.expiresDate);
  const planKey = transaction?.productId?.toUpperCase();
  if (!periodStartsAt || !periodEndsAt || !planKey) {
    return null;
  }

  return { periodStartsAt, periodEndsAt, planKey };
};

export const normalizeApple = ({ item, prior, now }: { item: AppleStatusItem; prior: IapPriorPeriod; now: dayjs.Dayjs }): NormalizedIap => {
  const { transaction, renewalInfo } = item;

  if (item.status === Status.REVOKED) {
    return { kind: 'expired', observed: observeApple(transaction) };
  }

  if (item.status === Status.EXPIRED) {
    if (isWithinStoreLagBuffer(prior, now)) {
      return { kind: 'defer', reason: 'apple-expired-within-store-lag-buffer' };
    }

    return { kind: 'expired', observed: observeApple(transaction) };
  }

  // 미지 상태를 약정보다 먼저 본다 — Google의 상태 전수 검사와 같은 자리라, 같은 입력 모양이 스토어별로
  // 다른 reason(=알람)을 내지 않는다.
  if (item.status !== Status.ACTIVE && item.status !== Status.BILLING_RETRY && item.status !== Status.BILLING_GRACE_PERIOD) {
    return { kind: 'unknown', reason: 'apple-status-unrecognized' };
  }

  // 약정은 결제 주기와 약정 종료를 동시에 반환해 현 모델로 표현되지 않는다 — 주기 산술을 하는 결과만 막고,
  // 산술이 없는 종결(회수)은 막지 않는다.
  if (transaction?.commitmentInfo || renewalInfo?.commitmentInfo) {
    return { kind: 'unknown', reason: 'apple-commitment-observed' };
  }

  const observed = observeApple(transaction);
  if (!observed) {
    return { kind: 'unknown', reason: 'apple-transaction-fields-missing' };
  }

  const intent = resolveAppleIntent(renewalInfo);

  let state: typeof SubscriptionState.ACTIVE | typeof SubscriptionState.WILL_EXPIRE | typeof SubscriptionState.IN_GRACE_PERIOD;
  if (item.status === Status.BILLING_GRACE_PERIOD) {
    state = SubscriptionState.IN_GRACE_PERIOD;
  } else if (intent === 'OFF' || item.status === Status.BILLING_RETRY) {
    state = SubscriptionState.WILL_EXPIRE;
  } else {
    state = SubscriptionState.ACTIVE;
  }

  return {
    kind: 'tracked',
    state,
    planKey: observed.planKey,
    periodStartsAt: observed.periodStartsAt,
    periodEndsAt: observed.periodEndsAt,
    intent,
    successorTokens: [],
    acknowledgePending: false,
    productId: null,
  };
};

// 스토어 확정 종료. 완충(defer)은 판정 보류이지 "살아 있음"이 아니다 — 승계 판정이 완충을 종료 부정으로 읽으면
// 만료 직후 완충 창 동안 정상 재가입이 독립 토큰으로 거절되고(등록) 재조정 승계도 하루 밀린다.
export const isAppleTerminated = (item: AppleStatusItem): boolean => item.status === Status.EXPIRED || item.status === Status.REVOKED;

export type AppleSuccessor =
  | { kind: 'none' }
  | { kind: 'succeeded'; originalTransactionId: string; normalized: NormalizedIap }
  | { kind: 'unresolved'; candidates: number };

// 응답에는 이미 후계 계약이 실려 있다 — 요청 ID 항목만 정규화하고 끝내면 승인된 새 계약이 canonical에 반영되지
// 않는 동안 구 항목이 tracked로 남는다. 후보 0건은 후계 없음(정상)이라 미해결이 아니다 — 매일 전 바인딩에서
// 미해결이 나오면 실제 실패가 묻힌다.
export const discoverAppleSuccessor = ({
  items,
  selected,
  requestedOriginalTransactionId,
  prior,
  now,
}: {
  items: AppleStatusItem[];
  selected: AppleStatusItem;
  requestedOriginalTransactionId: string;
  prior: IapPriorPeriod;
  now: dayjs.Dayjs;
}): AppleSuccessor => {
  const candidates = items.flatMap((item) => {
    const originalTransactionId = item.transaction?.originalTransactionId;
    if (
      !originalTransactionId ||
      originalTransactionId === requestedOriginalTransactionId ||
      !item.transaction?.appTransactionId ||
      item.transaction.appTransactionId !== selected.transaction?.appTransactionId ||
      !item.subscriptionGroupIdentifier ||
      item.subscriptionGroupIdentifier !== selected.subscriptionGroupIdentifier
    ) {
      return [];
    }

    // 매핑이 추적 가능할 때만 후계다 — 종료·미결제 매핑의 토큰이 canonical을 탈취하면 실제 계약이 추적에서 이탈한다.
    const normalized = normalizeApple({ item, prior, now });
    return normalized.kind === 'tracked' ? [{ originalTransactionId, normalized }] : [];
  });

  if (candidates.length === 0) {
    return { kind: 'none' };
  }

  // 확정 종료의 원천은 스토어 응답이다 — 로컬 상태만 보면 웹훅이 유실된 동안(재조정이 존재하는 이유) 이미 끝난
  // 계약이 살아 있는 것으로 보여 정상 승계가 하루 밀린다. 완충 창의 보류도 종료 부정이 아니다(같은 지연을 만든다).
  const terminated = prior?.state === SubscriptionState.EXPIRED || isAppleTerminated(selected);

  // 기존 계약이 스토어 확정 종료가 아닌데 추적 가능한 다른 계약이 있으면 잔여 계약의 오승계 위험이다 — 사람이 본다.
  if (!terminated || candidates.length > 1) {
    return { kind: 'unresolved', candidates: candidates.length };
  }

  return { kind: 'succeeded', ...candidates[0] };
};

const selectGoogleLineItem = (
  lineItems: GoogleLineItem[] | undefined,
): { kind: 'selected'; item: GoogleLineItem } | { kind: 'unknown'; reason: string } => {
  // 예약 항목은 소유 증거(latestSuccessfulOrderId)가 없다. 소유 증거만으로 고르면 발효된 지연 교체의
  // 만료된 구 항목이 후보로 남으므로 만료 시각이 가장 늦은 항목이 현재 서비스 항목이다.
  const owned = (lineItems ?? [])
    .map((item) => ({ item, expiry: parseTimestamp(item.expiryTime) }))
    .filter(
      (candidate): candidate is { item: GoogleLineItem; expiry: dayjs.Dayjs } =>
        !!candidate.item.latestSuccessfulOrderId && !!candidate.expiry,
    );
  if (owned.length === 0) {
    return { kind: 'unknown', reason: 'google-current-item-not-found' };
  }

  const latest = owned.reduce((best, candidate) => (candidate.expiry.isAfter(best.expiry) ? candidate : best));
  const tied = owned.filter((candidate) => candidate.expiry.isSame(latest.expiry));
  if (tied.length > 1) {
    return { kind: 'unknown', reason: 'google-current-item-ambiguous' };
  }

  return { kind: 'selected', item: latest.item };
};

const resolveGoogleIntent = (item: GoogleLineItem): IapRenewalIntent => {
  const hasAutoRenewingPlan = !!item.autoRenewingPlan;
  const hasPrepaidPlan = !!item.prepaidPlan;

  if (hasAutoRenewingPlan === hasPrepaidPlan) {
    return 'unknown';
  }
  if (hasPrepaidPlan) {
    return 'OFF';
  }
  if (item.autoRenewingPlan?.autoRenewEnabled === true) {
    return 'ON';
  }
  if (item.autoRenewingPlan?.autoRenewEnabled === false) {
    return 'OFF';
  }

  return 'unknown';
};

type GooglePeriod = { kind: 'ok'; periodStartsAt: dayjs.Dayjs; periodEndsAt: dayjs.Dayjs } | { kind: 'unknown'; reason: string };

// 선행 주기가 없는 최초 등록·복구의 시작. offer phase가 base 주기보다 짧으면 expiryTime이 phase의 종료라
// 역산이 주기 시작을 실구매 이전으로 보낸다 — phase 시작 = 구매 시각임이 확인될 때만 startTime을 쓴다.
// 도입 가격 phase는 base 주기로 반복될 수 있어 2회차 이후에는 startTime이 현재 주기 시작이 아니다.
const resolveGoogleStartFromOfferPhase = (
  item: GoogleLineItem,
  startTime: dayjs.Dayjs | null,
  expiry: dayjs.Dayjs,
  interval: PlanInterval,
): dayjs.Dayjs => {
  const backCalculated = subtractInterval(expiry, interval);
  if (!startTime) {
    return backCalculated;
  }

  const phase = item.offerPhase;
  if (phase?.freeTrial || phase?.prorationPeriod) {
    return startTime;
  }
  if (phase?.introductoryPrice) {
    return laterOf(startTime, backCalculated);
  }

  return backCalculated;
};

const resolveGooglePeriod = ({
  purchase,
  item,
  interval,
  prior,
  previousBoundaryAt,
}: {
  purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2;
  item: GoogleLineItem;
  interval: PlanInterval;
  prior: IapPriorPeriod;
  previousBoundaryAt: dayjs.Dayjs | undefined;
}): GooglePeriod => {
  const expiry = parseTimestamp(item.expiryTime);
  if (!expiry) {
    return { kind: 'unknown', reason: 'google-expiry-missing' };
  }

  const boundary = previousBoundaryAt ?? prior?.currentPeriodEndsAt ?? null;
  const startTime = parseTimestamp(purchase.startTime);

  // 유예 중의 근사 종료는 실제 경계와 어긋나 있어 전진 게이트가 회복된 주기를 동일 주기 연장으로 오판한다.
  // 교체 해석보다 먼저 두는 이유: 유예를 낀 교체에서 교체 규칙이 이기면 근사 주기가 그대로 굳는다.
  if (prior?.state === SubscriptionState.IN_GRACE_PERIOD) {
    return { kind: 'ok', periodStartsAt: subtractInterval(expiry, interval), periodEndsAt: expiry };
  }

  const replacementMode = item.itemReplacement?.replacementMode ?? null;
  if (replacementMode !== null) {
    if (replacementMode !== DEFERRED_MODE && !NEW_SERVICE_PERIOD_MODES.has(replacementMode) && !PLAN_ONLY_MODES.has(replacementMode)) {
      return { kind: 'unknown', reason: 'google-replacement-mode-unrecognized' };
    }

    // itemReplacement는 교체 후 60일간 응답에 남는다. 저장된 주기 시작이 교체 시각(새 토큰 부여) 이후면
    // 이미 반영된 교체이므로, 갱신 폴에서 다시 적용하면 지난 교체 시각이 거짓 주기 시작으로 되살아난다.
    if (NEW_SERVICE_PERIOD_MODES.has(replacementMode)) {
      const replacementApplied = !!prior && !!startTime && !prior.currentPeriodStartsAt.isBefore(startTime);

      if (!replacementApplied) {
        if (!startTime) {
          return { kind: 'unknown', reason: 'google-replacement-start-missing' };
        }

        return { kind: 'ok', periodStartsAt: startTime, periodEndsAt: expiry };
      }
    }

    // 주기·시작을 유지하는 모드에는 전용 분기를 두지 않는다 — 전진 게이트의 동일 주기 연장이 최초 관측에서
    // 같은 출력(시작 보존)을 내고, 전용 분기는 잔존 기간 내내 시작을 고정해 주기를 무한히 늘린다.
    if (replacementMode === DEFERRED_MODE) {
      // 이미 반영된 주기를 다시 폴하면(만료가 직전 종료를 넘지 못함) 시작을 그대로 둔다 — 두 산식 모두
      // 직전 종료에 밀려 시작이 종료와 같아지는 0길이 주기가 된다.
      if (prior && boundary && !expiry.isAfter(boundary)) {
        return { kind: 'ok', periodStartsAt: prior.currentPeriodStartsAt, periodEndsAt: expiry };
      }

      // 발효된 지연 교체의 새 항목에는 시작 필드가 없다. 구 항목의 만료가 전환 경계이고, 응답에서
      // 사라졌으면 직전 종료와 새 plan 기준 역산 중 늦은 쪽이다(top-level startTime은 구매를 누른 시각).
      // 구 항목도 잔존하므로 이미 지난 전환 경계는 직전 종료가 밀어낸다.
      const previousExpiry = (purchase.lineItems ?? [])
        .filter((other) => other !== item && !!other.latestSuccessfulOrderId)
        .map((other) => parseTimestamp(other.expiryTime))
        .filter((at): at is dayjs.Dayjs => !!at)
        .reduce<dayjs.Dayjs | null>((best, at) => (best === null || at.isAfter(best) ? at : best), null);

      if (previousExpiry) {
        return { kind: 'ok', periodStartsAt: boundary ? laterOf(boundary, previousExpiry) : previousExpiry, periodEndsAt: expiry };
      }

      const backCalculated = subtractInterval(expiry, interval);
      return { kind: 'ok', periodStartsAt: boundary ? laterOf(boundary, backCalculated) : backCalculated, periodEndsAt: expiry };
    }
  }

  if (boundary) {
    // 새 주기의 증거가 있을 때만 시작을 전진시킨다 — 미만은 동일 주기의 연장(결제 연기·유예)이라
    // 역산이 거짓 시작을 만든다. 갱신을 여럿 놓쳤으면 역산이, 하나만 놓쳤으면 직전 종료가 맞다.
    if (!expiry.isBefore(addInterval(boundary, interval))) {
      return { kind: 'ok', periodStartsAt: laterOf(boundary, subtractInterval(expiry, interval)), periodEndsAt: expiry };
    }

    if (prior) {
      return { kind: 'ok', periodStartsAt: prior.currentPeriodStartsAt, periodEndsAt: expiry };
    }
  }

  return { kind: 'ok', periodStartsAt: resolveGoogleStartFromOfferPhase(item, startTime, expiry, interval), periodEndsAt: expiry };
};

// 스토어 확정 종료(google). 완충(defer)을 섞지 않는 이유는 isAppleTerminated 참조. CANCELED 는 EXPIRED 로
// 뒤집히기 전이라도 만료 시각을 지났으면 종료다 — normalizeGoogle 의 만료 판정과 같은 식이다. 판정 불능
// (항목 미선택·만료 시각 결손)은 종료 부정으로 접는다 — 소비처는 종료 증거를 요구하는 쪽이라 부정이 안전하다.
export const isGoogleTerminated = (purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2, now: dayjs.Dayjs): boolean => {
  const state = purchase.subscriptionState ?? null;
  if (state === GOOGLE_STATE.EXPIRED) {
    return true;
  }
  if (state !== GOOGLE_STATE.CANCELED) {
    return false;
  }

  const selection = selectGoogleLineItem(purchase.lineItems);
  if (selection.kind !== 'selected') {
    return false;
  }

  const expiry = parseTimestamp(selection.item.expiryTime);
  return !!expiry && !expiry.isAfter(now);
};

export const normalizeGoogle = ({
  purchase,
  prior,
  planIntervals,
  previousBoundaryAt,
  now,
}: {
  purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2;
  prior: IapPriorPeriod;
  planIntervals: Record<string, PlanInterval>;
  previousBoundaryAt?: dayjs.Dayjs;
  now: dayjs.Dayjs;
}): NormalizedIap => {
  const state = purchase.subscriptionState ?? null;

  if (state === GOOGLE_STATE.PENDING) {
    return { kind: 'untracked', reason: 'pending' };
  }
  if (state === GOOGLE_STATE.PENDING_PURCHASE_CANCELED) {
    return { kind: 'untracked', reason: 'pending-canceled' };
  }

  const selection = selectGoogleLineItem(purchase.lineItems);
  const item = selection.kind === 'selected' ? selection.item : null;
  const planKey = item?.offerDetails?.basePlanId?.toUpperCase() ?? null;
  const interval = planKey === null ? undefined : planIntervals[planKey];
  const usable = interval === PlanInterval.MONTHLY || interval === PlanInterval.YEARLY;

  const observe = () => {
    if (!item || !planKey || !usable) {
      return null;
    }

    const period = resolveGooglePeriod({ purchase, item, interval, prior, previousBoundaryAt });
    if (period.kind !== 'ok') {
      return null;
    }

    return { periodStartsAt: period.periodStartsAt, periodEndsAt: period.periodEndsAt, planKey };
  };

  if (state === GOOGLE_STATE.EXPIRED) {
    if (isWithinStoreLagBuffer(prior, now)) {
      return { kind: 'defer', reason: 'google-expired-within-store-lag-buffer' };
    }

    return { kind: 'expired', observed: observe() };
  }

  if (
    state !== GOOGLE_STATE.ACTIVE &&
    state !== GOOGLE_STATE.CANCELED &&
    state !== GOOGLE_STATE.IN_GRACE_PERIOD &&
    state !== GOOGLE_STATE.ON_HOLD &&
    state !== GOOGLE_STATE.PAUSED
  ) {
    return { kind: 'unknown', reason: 'google-state-unrecognized' };
  }

  if (!item) {
    return { kind: 'unknown', reason: selection.kind === 'unknown' ? selection.reason : 'google-current-item-not-found' };
  }
  if (item.autoRenewingPlan?.installmentDetails) {
    return { kind: 'unknown', reason: 'google-installment-observed' };
  }
  if (!planKey) {
    return { kind: 'unknown', reason: 'google-base-plan-missing' };
  }
  if (!usable) {
    return { kind: 'unknown', reason: 'google-plan-interval-unsupported' };
  }

  const successorTokens = [purchase.linkedPurchaseToken, purchase.outOfAppPurchaseContext?.expiredPurchaseToken].filter(
    (token): token is string => !!token,
  );
  const acknowledgePending = purchase.acknowledgementState === ACKNOWLEDGEMENT_STATE_PENDING;
  const intent = resolveGoogleIntent(item);
  const tracked = {
    planKey,
    intent,
    successorTokens,
    acknowledgePending,
    productId: item.productId ?? null,
  };

  // 유예 중의 expiryTime은 유예 마감까지 동적으로 연장되므로 주기 종료가 아니다.
  if (state === GOOGLE_STATE.IN_GRACE_PERIOD) {
    return {
      kind: 'tracked',
      state: SubscriptionState.IN_GRACE_PERIOD,
      periodStartsAt: prior ? prior.currentPeriodStartsAt : subtractInterval(now, interval),
      periodEndsAt: prior ? prior.currentPeriodEndsAt : now,
      ...tracked,
    };
  }

  const expiry = parseTimestamp(item.expiryTime);
  if (!expiry) {
    return { kind: 'unknown', reason: 'google-expiry-missing' };
  }

  if (state === GOOGLE_STATE.CANCELED && !expiry.isAfter(now)) {
    return { kind: 'expired', observed: observe() };
  }

  const period = resolveGooglePeriod({ purchase, item, interval, prior, previousBoundaryAt });
  if (period.kind !== 'ok') {
    return { kind: 'unknown', reason: period.reason };
  }

  return {
    kind: 'tracked',
    state: intent !== 'OFF' && state === GOOGLE_STATE.ACTIVE ? SubscriptionState.ACTIVE : SubscriptionState.WILL_EXPIRE,
    periodStartsAt: period.periodStartsAt,
    periodEndsAt: period.periodEndsAt,
    ...tracked,
  };
};

export type IapEnrollBinding = { id: string; store: InAppPurchaseStore; canonical: EntitlementJudgmentRow | null };

export type IapEnrollPrecheck =
  | { allowed: true }
  | { allowed: false; reason: 'non-iap-subscription' | 'no-iap-plan' }
  | { allowed: false; reason: 'live-contract'; bindingIds: string[] };

export const precheckIapEnroll = ({
  rows,
  bindings,
  excludeBindingIds,
  iapPlanAvailable,
  now,
}: {
  rows: EntitlementSubscriptionRow[];
  bindings: IapEnrollBinding[];
  excludeBindingIds: string[];
  iapPlanAvailable: boolean;
  now: dayjs.Dayjs;
}): IapEnrollPrecheck => {
  const live = rows.filter((row) => isSubscriptionLive(row, now));

  if (live.some((row) => row.planAvailability !== PlanAvailability.IN_APP_PURCHASE && row.planAvailability !== PlanAvailability.TRIAL)) {
    return { allowed: false, reason: 'non-iap-subscription' };
  }

  const liveBindingIds = bindings
    .filter(
      (binding) => !excludeBindingIds.includes(binding.id) && binding.canonical !== null && isSubscriptionLive(binding.canonical, now),
    )
    .map((binding) => binding.id);

  if (liveBindingIds.length > 0) {
    return { allowed: false, reason: 'live-contract', bindingIds: liveBindingIds };
  }

  if (!iapPlanAvailable) {
    return { allowed: false, reason: 'no-iap-plan' };
  }

  return { allowed: true };
};

export type IapRegistrationOwnership =
  { kind: 'evidenced'; ownerUuid: string } | { kind: 'legacy' } | { kind: 'rejected'; reason: 'mismatch' | 'family-shared' };

// 스토어 payload 의 계정 식별자(appAccountToken·obfuscatedExternalAccountId)에서 등록 대상을 추출한다.
// 세션과의 대조가 아니다 — 결제한 계정과 로그인 계정이 달라도 등록은 소유자에게 귀속되므로, 여기서는
// 소유자를 하나로 지목할 수 있는지만 판정한다. 증거가 없는 구매(식별자 도입 전)는 legacy 로 남긴다.
export const extractIapRegistrationOwnership = ({
  apple,
  google,
}: {
  apple?: {
    transactionAppAccountToken: string | null;
    renewalAppAccountToken: string | null;
    renewalOriginalTransactionId: string | null;
    transactionOriginalTransactionId: string;
    requestedOriginalTransactionId: string;
    inAppOwnershipType: string | null;
  };
  google?: { obfuscatedExternalAccountId: string | null; expiredObfuscatedExternalAccountId: string | null };
}): IapRegistrationOwnership => {
  if (apple) {
    if (apple.inAppOwnershipType === InAppOwnershipType.FAMILY_SHARED) {
      return { kind: 'rejected', reason: 'family-shared' };
    }

    if (
      apple.renewalOriginalTransactionId !== null &&
      (apple.renewalOriginalTransactionId !== apple.transactionOriginalTransactionId ||
        apple.renewalOriginalTransactionId !== apple.requestedOriginalTransactionId)
    ) {
      return { kind: 'rejected', reason: 'mismatch' };
    }

    if (apple.transactionOriginalTransactionId !== apple.requestedOriginalTransactionId) {
      return { kind: 'rejected', reason: 'mismatch' };
    }

    // 두 서명 payload 모두 optional 이다. 둘 다 실렸는데 서로 다르면 소유자를 하나로 지목할 수 없다 — 임의로 고르지 않는다.
    const tokens = [apple.transactionAppAccountToken, apple.renewalAppAccountToken].filter((token): token is string => token !== null);
    if (tokens.length > 0) {
      if (new Set(tokens).size > 1) {
        return { kind: 'rejected', reason: 'mismatch' };
      }

      return { kind: 'evidenced', ownerUuid: tokens[0] };
    }
  }

  if (google) {
    const identifier = google.obfuscatedExternalAccountId ?? google.expiredObfuscatedExternalAccountId;
    if (identifier !== null) {
      return { kind: 'evidenced', ownerUuid: identifier };
    }
  }

  return { kind: 'legacy' };
};
