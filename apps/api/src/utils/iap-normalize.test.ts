import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import test from 'node:test';
import { AutoRenewStatus, Status } from '@apple/app-store-server-library';
import { InAppPurchaseStore, PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import {
  discoverAppleSuccessor,
  normalizeApple,
  normalizeGoogle,
  precheckIapEnroll,
  selectAppleStatusItem,
  validateIapRegistrationOwnership,
} from './iap-normalize.ts';
import type { JWSRenewalInfoDecodedPayload, JWSTransactionDecodedPayload } from '@apple/app-store-server-library';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';
import type { EntitlementSubscriptionRow } from './entitlement.ts';
import type { AppleStatusItem, IapPriorPeriod, NormalizedIap } from './iap-normalize.ts';

const now = dayjs('2026-08-15T03:00:00.000Z');

const iso = (at: dayjs.Dayjs) => at.toISOString();

const golden = (result: NormalizedIap): Record<string, unknown> => {
  if (result.kind === 'tracked') {
    return { ...result, periodStartsAt: iso(result.periodStartsAt), periodEndsAt: iso(result.periodEndsAt) };
  }

  if (result.kind === 'expired') {
    return {
      kind: 'expired',
      observed: result.observed
        ? { ...result.observed, periodStartsAt: iso(result.observed.periodStartsAt), periodEndsAt: iso(result.observed.periodEndsAt) }
        : null,
    };
  }

  return { ...result };
};

const prior = (over: Partial<Exclude<IapPriorPeriod, null>> = {}): IapPriorPeriod => ({
  state: SubscriptionState.ACTIVE,
  currentPeriodStartsAt: now.subtract(10, 'days'),
  currentPeriodEndsAt: now.add(20, 'days'),
  ...over,
});

const ORIGINAL_TRANSACTION_ID = 'OT-100';

const appleTransaction = (over: Partial<JWSTransactionDecodedPayload> = {}): JWSTransactionDecodedPayload => ({
  originalTransactionId: ORIGINAL_TRANSACTION_ID,
  transactionId: 'TX-1',
  productId: 'pl0fl1map',
  purchaseDate: now.subtract(10, 'days').valueOf(),
  expiresDate: now.add(20, 'days').valueOf(),
  signedDate: now.subtract(1, 'hour').valueOf(),
  ...over,
});

const appleRenewalInfo = (over: Partial<JWSRenewalInfoDecodedPayload> = {}): JWSRenewalInfoDecodedPayload => ({
  originalTransactionId: ORIGINAL_TRANSACTION_ID,
  autoRenewStatus: AutoRenewStatus.ON,
  ...over,
});

const appleItem = (over: Partial<AppleStatusItem> = {}): AppleStatusItem => ({
  status: Status.ACTIVE,
  outerOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
  transaction: appleTransaction(),
  renewalInfo: appleRenewalInfo(),
  subscriptionGroupIdentifier: 'SG-1',
  ...over,
});

const appleTracked = (over: Record<string, unknown> = {}): Record<string, unknown> => ({
  kind: 'tracked',
  state: SubscriptionState.ACTIVE,
  planKey: 'PL0FL1MAP',
  periodStartsAt: iso(now.subtract(10, 'days')),
  periodEndsAt: iso(now.add(20, 'days')),
  intent: 'ON',
  successorTokens: [],
  acknowledgePending: false,
  productId: null,
  ...over,
});

const googleLineItem = (
  over: Partial<androidpublisher_v3.Schema$SubscriptionPurchaseLineItem> = {},
): androidpublisher_v3.Schema$SubscriptionPurchaseLineItem => ({
  productId: 'plan.full',
  latestSuccessfulOrderId: 'GPA.1',
  offerDetails: { basePlanId: 'pl0fl1map' },
  expiryTime: iso(now.add(20, 'days')),
  autoRenewingPlan: { autoRenewEnabled: true },
  ...over,
});

const googlePurchase = (
  over: Partial<androidpublisher_v3.Schema$SubscriptionPurchaseV2> = {},
): androidpublisher_v3.Schema$SubscriptionPurchaseV2 => ({
  subscriptionState: 'SUBSCRIPTION_STATE_ACTIVE',
  lineItems: [googleLineItem()],
  acknowledgementState: 'ACKNOWLEDGEMENT_STATE_ACKNOWLEDGED',
  ...over,
});

const googleTracked = (over: Record<string, unknown> = {}): Record<string, unknown> => ({
  kind: 'tracked',
  state: SubscriptionState.ACTIVE,
  planKey: 'PL0FL1MAP',
  periodStartsAt: iso(now.subtract(10, 'days')),
  periodEndsAt: iso(now.add(20, 'days')),
  intent: 'ON',
  successorTokens: [],
  acknowledgePending: false,
  productId: 'plan.full',
  ...over,
});

const planIntervals = { PL0FL1MAP: PlanInterval.MONTHLY, PL0FL1YAP: PlanInterval.YEARLY };

test('애플 활성 + 자동갱신 ON은 ACTIVE로 정규화한다', () => {
  const result = normalizeApple({ item: appleItem(), prior: prior(), now });

  assert.deepEqual(golden(result), appleTracked());
});

test('애플 활성 + 자동갱신 OFF는 WILL_EXPIRE로 내린다', () => {
  const item = appleItem({ renewalInfo: appleRenewalInfo({ autoRenewStatus: AutoRenewStatus.OFF }) });
  const result = normalizeApple({ item, prior: prior(), now });

  assert.deepEqual(golden(result), appleTracked({ state: SubscriptionState.WILL_EXPIRE, intent: 'OFF' }));
});

test('애플 활성 + renewal info 부재는 의사 미상이어도 ACTIVE다 — 잠금 창을 만들지 않는다', () => {
  const result = normalizeApple({ item: appleItem({ renewalInfo: null }), prior: prior(), now });

  assert.deepEqual(golden(result), appleTracked({ intent: 'unknown' }));
});

test('애플 만료 + 로컬 주기 종료 12시간 경과는 종결을 보류한다', () => {
  const item = appleItem({
    status: Status.EXPIRED,
    transaction: appleTransaction({ purchaseDate: now.subtract(1, 'month').subtract(12, 'hours').valueOf() }),
  });
  const result = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(12, 'hours') }), now });

  assert.equal(result.kind, 'defer');

  const atBoundary = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(1, 'day') }), now });

  assert.equal(atBoundary.kind, 'defer');
});

test('애플 만료 + 로컬 주기 종료 3일 경과는 즉시 회수한다', () => {
  const item = appleItem({
    status: Status.EXPIRED,
    transaction: appleTransaction({
      purchaseDate: now.subtract(1, 'month').subtract(3, 'days').valueOf(),
      expiresDate: now.subtract(3, 'days').valueOf(),
    }),
  });
  const result = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(3, 'days') }), now });

  assert.deepEqual(golden(result), {
    kind: 'expired',
    observed: {
      periodStartsAt: iso(now.subtract(1, 'month').subtract(3, 'days')),
      periodEndsAt: iso(now.subtract(3, 'days')),
      planKey: 'PL0FL1MAP',
    },
  });
});

test('애플 만료 + 로컬 주기 종료가 미래면 중도 종료이므로 즉시 회수한다', () => {
  const result = normalizeApple({ item: appleItem({ status: Status.EXPIRED }), prior: prior(), now });

  assert.deepEqual(golden(result), {
    kind: 'expired',
    observed: {
      periodStartsAt: iso(now.subtract(10, 'days')),
      periodEndsAt: iso(now.add(20, 'days')),
      planKey: 'PL0FL1MAP',
    },
  });
});

test('애플 재청구 중은 WILL_EXPIRE이고 기간은 서명 거래 값 그대로다', () => {
  const item = appleItem({
    status: Status.BILLING_RETRY,
    transaction: appleTransaction({
      purchaseDate: now.subtract(1, 'month').subtract(2, 'days').valueOf(),
      expiresDate: now.subtract(2, 'days').valueOf(),
    }),
  });
  const result = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(2, 'days') }), now });

  assert.deepEqual(
    golden(result),
    appleTracked({
      state: SubscriptionState.WILL_EXPIRE,
      periodStartsAt: iso(now.subtract(1, 'month').subtract(2, 'days')),
      periodEndsAt: iso(now.subtract(2, 'days')),
    }),
  );
});

test('애플 유예는 IN_GRACE_PERIOD이고 유예 마감을 주기 종료로 쓰지 않는다', () => {
  const item = appleItem({
    status: Status.BILLING_GRACE_PERIOD,
    transaction: appleTransaction({
      purchaseDate: now.subtract(1, 'month').subtract(2, 'days').valueOf(),
      expiresDate: now.subtract(2, 'days').valueOf(),
    }),
    renewalInfo: appleRenewalInfo({ gracePeriodExpiresDate: now.add(14, 'days').valueOf() }),
  });
  const result = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(2, 'days') }), now });

  assert.deepEqual(
    golden(result),
    appleTracked({
      state: SubscriptionState.IN_GRACE_PERIOD,
      periodStartsAt: iso(now.subtract(1, 'month').subtract(2, 'days')),
      periodEndsAt: iso(now.subtract(2, 'days')),
    }),
  );
  assert.equal(result.kind === 'tracked' && result.periodEndsAt.isSame(now.add(14, 'days')), false);
});

test('애플 철회는 로컬 주기가 미래여도 즉시 회수한다', () => {
  const result = normalizeApple({ item: appleItem({ status: Status.REVOKED }), prior: prior(), now });

  assert.equal(result.kind, 'expired');
});

test('애플 미지 status는 전이를 보류한다 — 약정이 함께 관측돼도 상태 미지가 앞선다', () => {
  const result = normalizeApple({ item: appleItem({ status: 9 }), prior: prior(), now });

  assert.deepEqual(golden(result), { kind: 'unknown', reason: 'apple-status-unrecognized' });

  const withCommitment = normalizeApple({
    item: appleItem({ status: 9, transaction: appleTransaction({ commitmentInfo: { billingPeriodNumber: 1 } }) }),
    prior: prior(),
    now,
  });

  assert.deepEqual(golden(withCommitment), { kind: 'unknown', reason: 'apple-status-unrecognized' });
});

test('애플 항목 선택: outer 값이 없어도 서명 거래의 원거래 ID로 매칭한다', () => {
  const items = [appleItem({ outerOriginalTransactionId: null })];
  const result = selectAppleStatusItem(items, ORIGINAL_TRANSACTION_ID);

  assert.equal(result.kind, 'selected');
});

test('애플 항목 선택: outer와 서명 거래의 원거래 ID가 어긋나면 보류한다', () => {
  const items = [appleItem({ outerOriginalTransactionId: 'OT-OTHER' })];
  const result = selectAppleStatusItem(items, ORIGINAL_TRANSACTION_ID);

  assert.deepEqual(result, { kind: 'unknown', reason: 'apple-transaction-id-mismatch' });
});

test('애플 항목 선택: 복수 일치면 서명 시각이 가장 최신인 항목을 고른다', () => {
  const items = [
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-OLD', signedDate: now.subtract(2, 'days').valueOf() }) }),
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-NEW', signedDate: now.subtract(1, 'hour').valueOf() }) }),
  ];
  const result = selectAppleStatusItem(items, ORIGINAL_TRANSACTION_ID);

  assert.equal(result.kind, 'selected');
  assert.equal(result.kind === 'selected' ? result.item.transaction?.transactionId : null, 'TX-NEW');
});

test('애플 항목 선택: 서명 시각이 동률이면 보류한다', () => {
  const items = [
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-A' }) }),
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-B' }) }),
  ];
  const result = selectAppleStatusItem(items, ORIGINAL_TRANSACTION_ID);

  assert.equal(result.kind, 'unknown');
});

test('애플 항목 선택: 복수 일치 중 서명 시각이 없는 항목이 있으면 보류한다', () => {
  const items = [
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-A', signedDate: undefined }) }),
    appleItem({ transaction: appleTransaction({ transactionId: 'TX-B' }) }),
  ];
  const result = selectAppleStatusItem(items, ORIGINAL_TRANSACTION_ID);

  assert.deepEqual(result, { kind: 'unknown', reason: 'apple-signed-date-missing' });
});

test('애플 후계 발견: 후보가 없으면 미해결이 아니다', () => {
  const selected = appleItem({ transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const result = discoverAppleSuccessor({
    items: [selected],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.deepEqual(result, { kind: 'none' });
});

test('애플 후계 발견: 같은 앱·같은 그룹의 추적 가능한 단일 후보는 확정 종료 계약을 승계한다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const successor = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200', appTransactionId: 'APP-1' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, successor],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.equal(result.kind, 'succeeded');
  assert.equal(result.kind === 'succeeded' ? result.originalTransactionId : null, 'OT-200');
  assert.deepEqual(result.kind === 'succeeded' ? golden(result.normalized) : null, appleTracked());
});

test('애플 후계 발견: 로컬이 아직 살아있어도 선택 항목이 스토어 확정 종료면 승계한다 — 웹훅 유실이 승계를 막지 않는다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const successor = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200', appTransactionId: 'APP-1' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, successor],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior(),
    now,
  });

  assert.equal(result.kind, 'succeeded');
  assert.equal(result.kind === 'succeeded' ? result.originalTransactionId : null, 'OT-200');
});

test('애플 후계 발견: 완충 창 안의 스토어 만료도 확정 종료다 — 승계를 하루 미루지 않는다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const successor = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200', appTransactionId: 'APP-1' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, successor],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ currentPeriodStartsAt: now.subtract(30, 'days'), currentPeriodEndsAt: now.subtract(12, 'hours') }),
    now,
  });

  assert.equal(result.kind, 'succeeded');
  assert.equal(result.kind === 'succeeded' ? result.originalTransactionId : null, 'OT-200');
});

test('애플 후계 발견: 기존 계약이 스토어 확정 종료가 아니면 승계하지 않고 미해결이다', () => {
  const selected = appleItem({ transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const successor = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200', appTransactionId: 'APP-1' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, successor],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior(),
    now,
  });

  assert.deepEqual(result, { kind: 'unresolved', candidates: 1 });
});

test('애플 후계 발견: 추적 가능한 후보가 복수면 미해결이다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const successors = ['OT-200', 'OT-300'].map((originalTransactionId) =>
    appleItem({
      outerOriginalTransactionId: originalTransactionId,
      transaction: appleTransaction({ originalTransactionId, appTransactionId: 'APP-1' }),
      renewalInfo: appleRenewalInfo({ originalTransactionId }),
    }),
  );
  const result = discoverAppleSuccessor({
    items: [selected, ...successors],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.deepEqual(result, { kind: 'unresolved', candidates: 2 });
});

test('애플 후계 발견: 종료된 옛 계약은 후보가 아니다 — 재구독 이력이 매일 미해결을 만들지 않는다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const ended = appleItem({
    status: Status.EXPIRED,
    outerOriginalTransactionId: 'OT-050',
    transaction: appleTransaction({
      originalTransactionId: 'OT-050',
      appTransactionId: 'APP-1',
      purchaseDate: now.subtract(2, 'year').valueOf(),
      expiresDate: now.subtract(1, 'year').valueOf(),
    }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-050' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, ended],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.deepEqual(result, { kind: 'none' });
});

test('애플 후계 발견: 앱·그룹 증거가 어긋나는 항목은 후보가 아니다', () => {
  const selected = appleItem({ status: Status.EXPIRED, transaction: appleTransaction({ appTransactionId: 'APP-1' }) });
  const otherApp = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200', appTransactionId: 'APP-2' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const otherGroup = appleItem({
    outerOriginalTransactionId: 'OT-300',
    transaction: appleTransaction({ originalTransactionId: 'OT-300', appTransactionId: 'APP-1' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-300' }),
    subscriptionGroupIdentifier: 'SG-2',
  });
  const result = discoverAppleSuccessor({
    items: [selected, otherApp, otherGroup],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.deepEqual(result, { kind: 'none' });
});

test('애플 후계 발견: 앱 증거가 없으면 계약 lineage를 확인할 수 없으므로 후보가 아니다', () => {
  const selected = appleItem({ status: Status.EXPIRED });
  const successor = appleItem({
    outerOriginalTransactionId: 'OT-200',
    transaction: appleTransaction({ originalTransactionId: 'OT-200' }),
    renewalInfo: appleRenewalInfo({ originalTransactionId: 'OT-200' }),
  });
  const result = discoverAppleSuccessor({
    items: [selected, successor],
    selected,
    requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    prior: prior({ state: SubscriptionState.EXPIRED }),
    now,
  });

  assert.deepEqual(result, { kind: 'none' });
});

test('애플 서명 거래에 약정 정보가 있으면 일반화하지 않고 보류한다', () => {
  const item = appleItem({
    transaction: appleTransaction({ commitmentInfo: { billingPeriodNumber: 1, totalBillingPeriods: 12 } }),
  });
  const result = normalizeApple({ item, prior: prior(), now });

  assert.equal(result.kind, 'unknown');
});

test('애플 갱신 정보에 약정 정보가 있으면 일반화하지 않고 보류한다', () => {
  const item = appleItem({
    renewalInfo: appleRenewalInfo({ commitmentInfo: { commitmentRenewalDate: now.add(1, 'year').valueOf() } }),
  });
  const result = normalizeApple({ item, prior: prior(), now });

  assert.equal(result.kind, 'unknown');
});

test('애플 만료는 약정 정보가 관측돼도 회수를 막지 않는다 — 약정은 주기 산술만 게이트한다', () => {
  const item = appleItem({
    status: Status.EXPIRED,
    transaction: appleTransaction({
      purchaseDate: now.subtract(1, 'month').subtract(3, 'days').valueOf(),
      expiresDate: now.subtract(3, 'days').valueOf(),
      commitmentInfo: { billingPeriodNumber: 1, totalBillingPeriods: 12 },
    }),
  });
  const result = normalizeApple({ item, prior: prior({ currentPeriodEndsAt: now.subtract(3, 'days') }), now });

  assert.deepEqual(golden(result), {
    kind: 'expired',
    observed: {
      periodStartsAt: iso(now.subtract(1, 'month').subtract(3, 'days')),
      periodEndsAt: iso(now.subtract(3, 'days')),
      planKey: 'PL0FL1MAP',
    },
  });
});

test('구글 활성 + 자동갱신 true는 ACTIVE로 정규화한다', () => {
  const result = normalizeGoogle({ purchase: googlePurchase(), prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked());
});

test('구글 활성 + 자동갱신 false는 WILL_EXPIRE로 내린다', () => {
  const purchase = googlePurchase({ lineItems: [googleLineItem({ autoRenewingPlan: { autoRenewEnabled: false } })] });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ state: SubscriptionState.WILL_EXPIRE, intent: 'OFF' }));
});

test('구글 선불 항목은 구조적으로 갱신 의사 OFF다', () => {
  const purchase = googlePurchase({
    lineItems: [googleLineItem({ autoRenewingPlan: undefined, prepaidPlan: { allowExtendAfterTime: iso(now.add(1, 'day')) } })],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ state: SubscriptionState.WILL_EXPIRE, intent: 'OFF' }));
});

test('구글 활성 + 두 plan 필드 부재는 의사 미상이어도 ACTIVE다 — 잠금 창을 만들지 않는다', () => {
  const purchase = googlePurchase({ lineItems: [googleLineItem({ autoRenewingPlan: undefined })] });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ intent: 'unknown' }));
});

test('구글 활성 + 두 plan 필드 동시 존재는 판별 불가이므로 의사 미상이고 상태는 ACTIVE다', () => {
  const purchase = googlePurchase({
    lineItems: [googleLineItem({ prepaidPlan: { allowExtendAfterTime: iso(now.add(1, 'day')) } })],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ intent: 'unknown' }));
});

test('구글 해지 + 만료 미래는 WILL_EXPIRE로 등록·유지한다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_CANCELED',
    lineItems: [googleLineItem({ autoRenewingPlan: { autoRenewEnabled: false } })],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ state: SubscriptionState.WILL_EXPIRE, intent: 'OFF' }));
});

test('구글 해지 + 만료 과거는 즉시 회수한다 — 1일 보류는 만료 상태 전용이다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_CANCELED',
    lineItems: [googleLineItem({ autoRenewingPlan: { autoRenewEnabled: false }, expiryTime: iso(now.subtract(12, 'hours')) })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(12, 'hours'), currentPeriodEndsAt: now.subtract(12, 'hours') }),
    planIntervals,
    now,
  });

  assert.deepEqual(golden(result), {
    kind: 'expired',
    observed: {
      periodStartsAt: iso(now.subtract(1, 'month').subtract(12, 'hours')),
      periodEndsAt: iso(now.subtract(12, 'hours')),
      planKey: 'PL0FL1MAP',
    },
  });
});

test('구글 유예는 연장된 만료를 버리고 직전 주기를 유지한다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_IN_GRACE_PERIOD',
    lineItems: [googleLineItem({ expiryTime: iso(now.add(7, 'days')) })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(40, 'days'), currentPeriodEndsAt: now.subtract(10, 'days') }),
    planIntervals,
    now,
  });

  assert.deepEqual(
    golden(result),
    googleTracked({
      state: SubscriptionState.IN_GRACE_PERIOD,
      periodStartsAt: iso(now.subtract(40, 'days')),
      periodEndsAt: iso(now.subtract(10, 'days')),
    }),
  );
});

test('구글 유예 중 최초 등록은 0길이 주기를 만들지 않고 근사한다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_IN_GRACE_PERIOD',
    lineItems: [googleLineItem({ expiryTime: iso(now.add(7, 'days')) })],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({
      state: SubscriptionState.IN_GRACE_PERIOD,
      periodStartsAt: iso(now.subtract(1, 'month')),
      periodEndsAt: iso(now),
    }),
  );
});

test('구글 계정 보류는 WILL_EXPIRE이고 기간을 훼손하지 않는다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_ON_HOLD',
    lineItems: [googleLineItem({ expiryTime: iso(now.subtract(3, 'days')) })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(3, 'days'), currentPeriodEndsAt: now.subtract(3, 'days') }),
    planIntervals,
    now,
  });

  assert.deepEqual(
    golden(result),
    googleTracked({
      state: SubscriptionState.WILL_EXPIRE,
      periodStartsAt: iso(now.subtract(1, 'month').subtract(3, 'days')),
      periodEndsAt: iso(now.subtract(3, 'days')),
    }),
  );
});

test('구글 일시중지는 WILL_EXPIRE다', () => {
  const purchase = googlePurchase({
    subscriptionState: 'SUBSCRIPTION_STATE_PAUSED',
    lineItems: [googleLineItem({ expiryTime: iso(now.subtract(3, 'days')) })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(3, 'days'), currentPeriodEndsAt: now.subtract(3, 'days') }),
    planIntervals,
    now,
  });

  assert.deepEqual(
    golden(result),
    googleTracked({
      state: SubscriptionState.WILL_EXPIRE,
      periodStartsAt: iso(now.subtract(1, 'month').subtract(3, 'days')),
      periodEndsAt: iso(now.subtract(3, 'days')),
    }),
  );
});

test('구글 만료는 로컬 주기 종료 1일 이내면 보류, 경과·미래면 즉시 회수한다', () => {
  const expiredPurchase = (expiryTime: string) =>
    googlePurchase({ subscriptionState: 'SUBSCRIPTION_STATE_EXPIRED', lineItems: [googleLineItem({ expiryTime })] });

  const within = normalizeGoogle({
    purchase: expiredPurchase(iso(now.subtract(12, 'hours'))),
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(12, 'hours'), currentPeriodEndsAt: now.subtract(12, 'hours') }),
    planIntervals,
    now,
  });
  assert.equal(within.kind, 'defer');

  const elapsed = normalizeGoogle({
    purchase: expiredPurchase(iso(now.subtract(3, 'days'))),
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(3, 'days'), currentPeriodEndsAt: now.subtract(3, 'days') }),
    planIntervals,
    now,
  });
  assert.deepEqual(golden(elapsed), {
    kind: 'expired',
    observed: {
      periodStartsAt: iso(now.subtract(1, 'month').subtract(3, 'days')),
      periodEndsAt: iso(now.subtract(3, 'days')),
      planKey: 'PL0FL1MAP',
    },
  });

  const future = normalizeGoogle({ purchase: expiredPurchase(iso(now.add(20, 'days'))), prior: prior(), planIntervals, now });
  assert.deepEqual(golden(future), {
    kind: 'expired',
    observed: { periodStartsAt: iso(now.subtract(10, 'days')), periodEndsAt: iso(now.add(20, 'days')), planKey: 'PL0FL1MAP' },
  });
});

test('구글 결제 대기는 추적 대상이 아니다', () => {
  const purchase = googlePurchase({ subscriptionState: 'SUBSCRIPTION_STATE_PENDING', lineItems: [] });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(golden(result), { kind: 'untracked', reason: 'pending' });
});

test('구글 결제 대기 취소는 추적 대상이 아니다', () => {
  const purchase = googlePurchase({ subscriptionState: 'SUBSCRIPTION_STATE_PENDING_PURCHASE_CANCELED', lineItems: [] });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(golden(result), { kind: 'untracked', reason: 'pending-canceled' });
});

test('구글 미지정 상태는 전이를 보류한다', () => {
  const purchase = googlePurchase({ subscriptionState: 'SUBSCRIPTION_STATE_UNSPECIFIED' });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.equal(result.kind, 'unknown');
});

test('구글 항목 선택: 소유 증거가 없는 예약 항목은 제외한다', () => {
  const purchase = googlePurchase({
    lineItems: [
      googleLineItem({
        latestSuccessfulOrderId: undefined,
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(1, 'year')),
      }),
      googleLineItem(),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked());
});

test('구글 항목 선택: 만료 시각까지 같은 소유 항목이 복수면 보류한다', () => {
  const purchase = googlePurchase({
    lineItems: [googleLineItem(), googleLineItem({ latestSuccessfulOrderId: 'GPA.2', offerDetails: { basePlanId: 'pl0fl1yap' } })],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), { kind: 'unknown', reason: 'google-current-item-ambiguous' });
});

test('구글 항목 선택: 소유 증거를 가진 후보가 없으면 보류한다', () => {
  const purchase = googlePurchase({ lineItems: [googleLineItem({ latestSuccessfulOrderId: undefined })] });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), { kind: 'unknown', reason: 'google-current-item-not-found' });
});

test('구글 전진 게이트: 새 만료가 직전 종료 + 한 주기 이상이면 시작을 전진시킨다', () => {
  const renewed = normalizeGoogle({
    purchase: googlePurchase({ lineItems: [googleLineItem({ expiryTime: iso(now.subtract(1, 'day').add(1, 'month')) })] }),
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month').subtract(1, 'day'), currentPeriodEndsAt: now.subtract(1, 'day') }),
    planIntervals,
    now,
  });
  assert.deepEqual(
    golden(renewed),
    googleTracked({ periodStartsAt: iso(now.subtract(1, 'day')), periodEndsAt: iso(now.subtract(1, 'day').add(1, 'month')) }),
  );

  const missedSeveral = normalizeGoogle({
    purchase: googlePurchase({ lineItems: [googleLineItem({ expiryTime: iso(now.add(1, 'day')) })] }),
    prior: prior({ currentPeriodStartsAt: now.subtract(4, 'months'), currentPeriodEndsAt: now.subtract(3, 'months') }),
    planIntervals,
    now,
  });
  assert.deepEqual(
    golden(missedSeveral),
    googleTracked({ periodStartsAt: iso(now.add(1, 'day').subtract(1, 'month')), periodEndsAt: iso(now.add(1, 'day')) }),
  );
});

test('구글 동일 주기 연장은 시작을 보존하고 종료만 갱신한다', () => {
  const purchase = googlePurchase({ lineItems: [googleLineItem({ expiryTime: iso(now.add(27, 'days')) })] });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ periodStartsAt: iso(now.subtract(10, 'days')), periodEndsAt: iso(now.add(27, 'days')) }),
  );
});

test('구글 즉시 교체(시간 비례)는 새 서비스 구간을 시작한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(2, 'days')),
    lineItems: [
      googleLineItem({
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(1, 'year')),
        itemReplacement: { replacementMode: 'WITH_TIME_PRORATION' },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(2, 'days')), periodEndsAt: iso(now.add(1, 'year')) }),
  );
});

test('구글 즉시 교체(전액 청구)는 새 서비스 구간을 시작한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(2, 'days')),
    lineItems: [
      googleLineItem({
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(1, 'year')),
        itemReplacement: { replacementMode: 'CHARGE_FULL_PRICE' },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(2, 'days')), periodEndsAt: iso(now.add(1, 'year')) }),
  );
});

test('구글 즉시 교체(비례 가격 청구)는 플랜만 바꾸고 주기·시작을 유지한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(2, 'days')),
    lineItems: [
      googleLineItem({ offerDetails: { basePlanId: 'pl0fl1yap' }, itemReplacement: { replacementMode: 'CHARGE_PRORATED_PRICE' } }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ planKey: 'PL0FL1YAP' }));
});

test('구글 즉시 교체(비례 없음)는 플랜만 바꾸고 주기·시작을 유지한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(2, 'days')),
    lineItems: [googleLineItem({ offerDetails: { basePlanId: 'pl0fl1yap' }, itemReplacement: { replacementMode: 'WITHOUT_PRORATION' } })],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ planKey: 'PL0FL1YAP' }));
});

test('구글 지연 교체는 발효 전까지 현재 플랜·주기를 유지한다', () => {
  const purchase = googlePurchase({
    lineItems: [
      googleLineItem({ deferredItemReplacement: { productId: 'plan.full' } }),
      googleLineItem({
        latestSuccessfulOrderId: undefined,
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(1, 'year')),
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked());
});

test('구글 지연 교체 발효 + 구 항목 잔존이면 새 시작은 구 항목의 만료 시각이다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(1, 'year')),
    lineItems: [
      googleLineItem({ expiryTime: iso(now.subtract(1, 'hour')) }),
      googleLineItem({
        latestSuccessfulOrderId: 'GPA.2',
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(11, 'months')),
        itemReplacement: { replacementMode: 'DEFERRED' },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior({ currentPeriodEndsAt: now.subtract(10, 'days') }), planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({
      planKey: 'PL0FL1YAP',
      periodStartsAt: iso(now.subtract(1, 'hour')),
      periodEndsAt: iso(now.add(11, 'months')),
    }),
  );
});

test('구글 지연 교체 발효 + 구 항목 소실이면 직전 종료와 역산 중 늦은 쪽을 시작으로 쓴다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(1, 'year')),
    lineItems: [
      googleLineItem({
        latestSuccessfulOrderId: 'GPA.2',
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(1, 'year')),
        itemReplacement: { replacementMode: 'DEFERRED' },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior({ currentPeriodEndsAt: now.subtract(10, 'days') }), planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now), periodEndsAt: iso(now.add(1, 'year')) }),
  );
});

test('구글 지연 교체가 반영된 뒤 같은 주기를 다시 폴해도 시작은 변하지 않는다 — 0길이 주기 금지', () => {
  const previousItem = googleLineItem({ expiryTime: iso(now.subtract(1, 'hour')) });
  const currentItem = googleLineItem({
    latestSuccessfulOrderId: 'GPA.2',
    offerDetails: { basePlanId: 'pl0fl1yap' },
    expiryTime: iso(now.add(11, 'months')),
    itemReplacement: { replacementMode: 'DEFERRED' },
  });
  const applied = prior({ currentPeriodStartsAt: now.subtract(1, 'hour'), currentPeriodEndsAt: now.add(11, 'months') });

  const withPreviousItem = normalizeGoogle({
    purchase: googlePurchase({ startTime: iso(now.subtract(1, 'year')), lineItems: [previousItem, currentItem] }),
    prior: applied,
    planIntervals,
    now,
  });
  assert.deepEqual(
    golden(withPreviousItem),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(1, 'hour')), periodEndsAt: iso(now.add(11, 'months')) }),
  );

  const withoutPreviousItem = normalizeGoogle({
    purchase: googlePurchase({ startTime: iso(now.subtract(1, 'year')), lineItems: [currentItem] }),
    prior: applied,
    planIntervals,
    now,
  });
  assert.deepEqual(
    golden(withoutPreviousItem),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(1, 'hour')), periodEndsAt: iso(now.add(11, 'months')) }),
  );
});

test('구글 미지·유지 교체 모드는 전이를 보류한다', () => {
  const withMode = (replacementMode: string) =>
    normalizeGoogle({
      purchase: googlePurchase({ lineItems: [googleLineItem({ itemReplacement: { replacementMode } })] }),
      prior: prior(),
      planIntervals,
      now,
    });

  const expected = { kind: 'unknown', reason: 'google-replacement-mode-unrecognized' };
  assert.deepEqual(golden(withMode('REPLACEMENT_MODE_UNSPECIFIED')), expected);
  assert.deepEqual(golden(withMode('KEEP_EXISTING')), expected);
  assert.deepEqual(golden(withMode('SOME_FUTURE_MODE')), expected);
});

test('구글 즉시 교체가 이미 반영된 뒤의 갱신 폴은 지난 교체 시각을 재적용하지 않는다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(1, 'month')),
    lineItems: [googleLineItem({ expiryTime: iso(now.add(1, 'month')), itemReplacement: { replacementMode: 'WITH_TIME_PRORATION' } })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(1, 'month'), currentPeriodEndsAt: now }),
    planIntervals,
    now,
  });

  assert.deepEqual(golden(result), googleTracked({ periodStartsAt: iso(now), periodEndsAt: iso(now.add(1, 'month')) }));
});

test('구글 플랜만 교체는 갱신 전 폴에서 시작을 보존하고 갱신 폴이면 전진시킨다', () => {
  // 주기·시작을 유지하는 교체라 저장된 주기 시작은 교체 시각보다 늘 앞이다 — 시퀀스로 고정한다.
  const replacedAt = now.subtract(2, 'months').add(10, 'days');
  const purchase = (expiryTime: string) =>
    googlePurchase({
      startTime: iso(replacedAt),
      lineItems: [googleLineItem({ expiryTime, itemReplacement: { replacementMode: 'CHARGE_PRORATED_PRICE' } })],
    });

  const beforeRenewal = normalizeGoogle({
    purchase: purchase(iso(now.subtract(1, 'month'))),
    prior: prior({ currentPeriodStartsAt: now.subtract(2, 'months'), currentPeriodEndsAt: now.subtract(1, 'month') }),
    planIntervals,
    now,
  });
  assert.deepEqual(
    golden(beforeRenewal),
    googleTracked({ periodStartsAt: iso(now.subtract(2, 'months')), periodEndsAt: iso(now.subtract(1, 'month')) }),
  );

  const renewed = normalizeGoogle({
    purchase: purchase(iso(now)),
    prior: prior({ currentPeriodStartsAt: now.subtract(2, 'months'), currentPeriodEndsAt: now.subtract(1, 'month') }),
    planIntervals,
    now,
  });
  assert.deepEqual(golden(renewed), googleTracked({ periodStartsAt: iso(now.subtract(1, 'month')), periodEndsAt: iso(now) }));
});

test('구글 신규 등록의 무료 체험 단계는 부여 시각을 주기 시작으로 쓴다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(3, 'days')),
    lineItems: [googleLineItem({ expiryTime: iso(now.add(4, 'days')), offerPhase: { freeTrial: {} } })],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ periodStartsAt: iso(now.subtract(3, 'days')), periodEndsAt: iso(now.add(4, 'days')) }));
});

test('구글 복구의 도입 가격 반복 단계는 부여 시각이 아니라 역산으로 주기 시작을 정한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(2, 'months').subtract(5, 'days')),
    lineItems: [googleLineItem({ expiryTime: iso(now.add(25, 'days')), offerPhase: { introductoryPrice: {} } })],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ periodStartsAt: iso(now.add(25, 'days').subtract(1, 'month')), periodEndsAt: iso(now.add(25, 'days')) }),
  );
});

test('구글 복구의 기본 가격 단계도 역산으로 주기 시작을 정한다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(8, 'months')),
    lineItems: [googleLineItem({ expiryTime: iso(now.add(10, 'days')), offerPhase: { basePrice: {} } })],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ periodStartsAt: iso(now.add(10, 'days').subtract(1, 'month')), periodEndsAt: iso(now.add(10, 'days')) }),
  );
});

test('구글 단일 회차 도입 가격 단계는 부여 시각을 주기 시작으로 쓴다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(5, 'days')),
    lineItems: [
      googleLineItem({
        offerDetails: { basePlanId: 'pl0fl1yap' },
        expiryTime: iso(now.add(25, 'days')),
        offerPhase: { introductoryPrice: {} },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(
    golden(result),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(5, 'days')), periodEndsAt: iso(now.add(25, 'days')) }),
  );
});

test('구글 비례 기간 단계는 교체 시각을 주기 시작으로 쓴다', () => {
  const purchase = googlePurchase({
    startTime: iso(now.subtract(3, 'days')),
    lineItems: [
      googleLineItem({ expiryTime: iso(now.add(12, 'days')), offerPhase: { prorationPeriod: { originalOfferPhaseType: 'BASE_PRICE' } } }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: null, planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ periodStartsAt: iso(now.subtract(3, 'days')), periodEndsAt: iso(now.add(12, 'days')) }));
});

test('구글 유예 해소 후 첫 비유예 응답은 전진 게이트를 우회해 무조건 역산한다', () => {
  const purchase = googlePurchase({ lineItems: [googleLineItem({ expiryTime: iso(now.add(20, 'days')) })] });
  const result = normalizeGoogle({
    purchase,
    prior: prior({
      state: SubscriptionState.IN_GRACE_PERIOD,
      currentPeriodStartsAt: now.subtract(31, 'days'),
      currentPeriodEndsAt: now.subtract(1, 'day'),
    }),
    planIntervals,
    now,
  });

  assert.deepEqual(
    golden(result),
    googleTracked({ periodStartsAt: iso(now.add(20, 'days').subtract(1, 'month')), periodEndsAt: iso(now.add(20, 'days')) }),
  );
});

test('구글 할부 약정이 관측되면 일반화하지 않고 보류한다', () => {
  const purchase = googlePurchase({
    lineItems: [
      googleLineItem({
        autoRenewingPlan: { autoRenewEnabled: true, installmentDetails: { initialCommittedPaymentsCount: 12 } },
      }),
    ],
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.equal(result.kind, 'unknown');
});

test('구글 승계 신호는 연결 토큰과 만료 토큰을 모두 수집한다', () => {
  const purchase = googlePurchase({
    linkedPurchaseToken: 'TOKEN-LINKED',
    outOfAppPurchaseContext: { expiredPurchaseToken: 'TOKEN-EXPIRED' },
  });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ successorTokens: ['TOKEN-LINKED', 'TOKEN-EXPIRED'] }));
});

test('구글 미승인 토큰은 acknowledge 백스톱 대상으로 노출한다', () => {
  const purchase = googlePurchase({ acknowledgementState: 'ACKNOWLEDGEMENT_STATE_PENDING' });
  const result = normalizeGoogle({ purchase, prior: prior(), planIntervals, now });

  assert.deepEqual(golden(result), googleTracked({ acknowledgePending: true }));
});

test('구글 월간↔연간 변경의 역산은 현재 항목 basePlanId가 가리키는 플랜의 주기를 쓴다', () => {
  const purchase = googlePurchase({
    lineItems: [googleLineItem({ offerDetails: { basePlanId: 'pl0fl1yap' }, expiryTime: iso(now.add(10, 'months')) })],
  });
  const result = normalizeGoogle({
    purchase,
    prior: prior({ currentPeriodStartsAt: now.subtract(4, 'months'), currentPeriodEndsAt: now.subtract(3, 'months') }),
    planIntervals,
    now,
  });

  assert.deepEqual(
    golden(result),
    googleTracked({ planKey: 'PL0FL1YAP', periodStartsAt: iso(now.subtract(2, 'months')), periodEndsAt: iso(now.add(10, 'months')) }),
  );
});

test('백필 호출: 오염 시작은 교정하고 정상 시작·유예 주기는 보존하며 원장 경계 재적용은 멱등이다', () => {
  const activePurchase = (expiryTime: string) => googlePurchase({ lineItems: [googleLineItem({ expiryTime })] });

  const polluted = normalizeGoogle({
    purchase: activePurchase(iso(now.add(10, 'days'))),
    prior: prior({ currentPeriodStartsAt: now.subtract(8, 'months'), currentPeriodEndsAt: now.add(10, 'days') }),
    planIntervals,
    previousBoundaryAt: now.subtract(8, 'months'),
    now,
  });
  assert.deepEqual(
    golden(polluted),
    googleTracked({ periodStartsAt: iso(now.add(10, 'days').subtract(1, 'month')), periodEndsAt: iso(now.add(10, 'days')) }),
  );

  const intact = normalizeGoogle({
    purchase: activePurchase(iso(now.add(10, 'days'))),
    prior: prior({ currentPeriodStartsAt: now.add(10, 'days').subtract(1, 'month'), currentPeriodEndsAt: now.add(10, 'days') }),
    planIntervals,
    previousBoundaryAt: now.add(10, 'days').subtract(1, 'month'),
    now,
  });
  assert.deepEqual(
    golden(intact),
    googleTracked({ periodStartsAt: iso(now.add(10, 'days').subtract(1, 'month')), periodEndsAt: iso(now.add(10, 'days')) }),
  );

  const grace = normalizeGoogle({
    purchase: googlePurchase({
      subscriptionState: 'SUBSCRIPTION_STATE_IN_GRACE_PERIOD',
      lineItems: [googleLineItem({ expiryTime: iso(now.add(5, 'days')) })],
    }),
    prior: prior({
      state: SubscriptionState.IN_GRACE_PERIOD,
      currentPeriodStartsAt: now.subtract(40, 'days'),
      currentPeriodEndsAt: now.subtract(10, 'days'),
    }),
    planIntervals,
    previousBoundaryAt: now.subtract(40, 'days'),
    now,
  });
  assert.deepEqual(
    golden(grace),
    googleTracked({
      state: SubscriptionState.IN_GRACE_PERIOD,
      periodStartsAt: iso(now.subtract(40, 'days')),
      periodEndsAt: iso(now.subtract(10, 'days')),
    }),
  );

  const expired = normalizeGoogle({
    purchase: googlePurchase({
      subscriptionState: 'SUBSCRIPTION_STATE_EXPIRED',
      lineItems: [googleLineItem({ expiryTime: iso(now.subtract(1, 'month')) })],
    }),
    prior: prior({ currentPeriodStartsAt: now.subtract(2, 'months'), currentPeriodEndsAt: now.subtract(1, 'month') }),
    planIntervals,
    previousBoundaryAt: now.subtract(2, 'months'),
    now,
  });
  assert.deepEqual(golden(expired), {
    kind: 'expired',
    observed: { periodStartsAt: iso(now.subtract(2, 'months')), periodEndsAt: iso(now.subtract(1, 'month')), planKey: 'PL0FL1MAP' },
  });

  const reapplied = normalizeGoogle({
    purchase: activePurchase(iso(now.add(10, 'days'))),
    prior: prior({ currentPeriodStartsAt: now.add(10, 'days').subtract(1, 'month'), currentPeriodEndsAt: now.add(10, 'days') }),
    planIntervals,
    previousBoundaryAt: now.subtract(8, 'months'),
    now,
  });
  assert.deepEqual(golden(reapplied), golden(polluted));
});

const enrollRow = (over: Partial<EntitlementSubscriptionRow> = {}): EntitlementSubscriptionRow => ({
  id: 'sub_1',
  state: SubscriptionState.ACTIVE,
  planAvailability: PlanAvailability.BILLING_KEY,
  startsAt: now.subtract(60, 'days'),
  currentPeriodStartsAt: now.subtract(10, 'days'),
  currentPeriodEndsAt: now.add(20, 'days'),
  createdAt: now.subtract(60, 'days'),
  ...over,
});

test('등록 사전 판정: 다른 스토어 바인딩이 있으면 계약 상태와 무관하게 거절한다', () => {
  const withRows = (rows: EntitlementSubscriptionRow[]) =>
    precheckIapEnroll({
      rows,
      binding: { store: InAppPurchaseStore.APP_STORE },
      store: InAppPurchaseStore.GOOGLE_PLAY,
      iapPlanAvailable: true,
      now,
    });

  assert.deepEqual(withRows([]), { allowed: false, reason: 'cross-store-binding' });
  assert.deepEqual(withRows([enrollRow({ state: SubscriptionState.EXPIRED, planAvailability: PlanAvailability.IN_APP_PURCHASE })]), {
    allowed: false,
    reason: 'cross-store-binding',
  });
});

test('등록 사전 판정: 살아있는 비-IAP·비-트라이얼 구독이 있으면 거절한다', () => {
  const result = precheckIapEnroll({
    rows: [enrollRow()],
    binding: null,
    store: InAppPurchaseStore.GOOGLE_PLAY,
    iapPlanAvailable: true,
    now,
  });

  assert.deepEqual(result, { allowed: false, reason: 'non-iap-subscription' });
});

test('등록 사전 판정: 판매 중인 IAP 플랜이 없으면 거절한다', () => {
  const result = precheckIapEnroll({
    rows: [],
    binding: null,
    store: InAppPurchaseStore.GOOGLE_PLAY,
    iapPlanAvailable: false,
    now,
  });

  assert.deepEqual(result, { allowed: false, reason: 'no-iap-plan' });
});

test('등록 사전 판정: 같은 스토어 바인딩·트라이얼·기간 경과 행은 등록을 막지 않는다', () => {
  const allowedWith = (rows: EntitlementSubscriptionRow[]) =>
    precheckIapEnroll({
      rows,
      binding: { store: InAppPurchaseStore.GOOGLE_PLAY },
      store: InAppPurchaseStore.GOOGLE_PLAY,
      iapPlanAvailable: true,
      now,
    });

  assert.deepEqual(allowedWith([enrollRow({ planAvailability: PlanAvailability.IN_APP_PURCHASE })]), { allowed: true });
  assert.deepEqual(allowedWith([enrollRow({ planAvailability: PlanAvailability.TRIAL })]), { allowed: true });
  assert.deepEqual(allowedWith([enrollRow({ state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now.subtract(1, 'minute') })]), {
    allowed: true,
  });
  assert.deepEqual(allowedWith([enrollRow({ state: SubscriptionState.WILL_ACTIVATE })]), { allowed: true });
  assert.deepEqual(allowedWith([enrollRow({ state: SubscriptionState.EXPIRED })]), { allowed: true });
});

const SESSION_UUID = '018f0000-0000-7000-8000-0000000000aa';
const OTHER_UUID = '018f0000-0000-7000-8000-0000000000bb';

const appleOwnership = (over: Record<string, unknown> = {}) => ({
  transactionAppAccountToken: null,
  renewalAppAccountToken: null,
  renewalOriginalTransactionId: null,
  transactionOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
  requestedOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
  inAppOwnershipType: 'PURCHASED',
  ...over,
});

test('소유 증거: 애플 거래 토큰만 있고 세션과 일치하면 수용한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ transactionAppAccountToken: SESSION_UUID }),
  });

  assert.deepEqual(result, { kind: 'accepted', legacy: false });
});

test('소유 증거: 애플 갱신 토큰만 있어도 레거시로 통과시키지 않고 대조한다', () => {
  const matched = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ renewalAppAccountToken: SESSION_UUID }),
  });
  assert.deepEqual(matched, { kind: 'accepted', legacy: false });

  const mismatched = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ renewalAppAccountToken: OTHER_UUID }),
  });
  assert.deepEqual(mismatched, { kind: 'rejected', reason: 'mismatch' });
});

test('소유 증거: 애플 양쪽 토큰이 세션·서로 일치하면 수용한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({
      transactionAppAccountToken: SESSION_UUID,
      renewalAppAccountToken: SESSION_UUID,
      renewalOriginalTransactionId: ORIGINAL_TRANSACTION_ID,
    }),
  });

  assert.deepEqual(result, { kind: 'accepted', legacy: false });
});

test('소유 증거: 애플 양쪽 토큰이 서로 어긋나면 거절한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ transactionAppAccountToken: SESSION_UUID, renewalAppAccountToken: OTHER_UUID }),
  });

  assert.deepEqual(result, { kind: 'rejected', reason: 'mismatch' });
});

test('소유 증거: 애플 갱신 정보의 원거래 ID가 거래·요청 값과 다르면 거절한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ transactionAppAccountToken: SESSION_UUID, renewalOriginalTransactionId: 'OT-999' }),
  });

  assert.deepEqual(result, { kind: 'rejected', reason: 'mismatch' });
});

test('소유 증거: 애플 토큰이 양쪽 모두 없을 때만 레거시로 통과시킨다', () => {
  const result = validateIapRegistrationOwnership({ sessionUuid: SESSION_UUID, apple: appleOwnership() });

  assert.deepEqual(result, { kind: 'accepted', legacy: true });
});

test('소유 증거: 애플 가족 공유 거래는 거절한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    apple: appleOwnership({ transactionAppAccountToken: SESSION_UUID, inAppOwnershipType: 'FAMILY_SHARED' }),
  });

  assert.deepEqual(result, { kind: 'rejected', reason: 'family-shared' });
});

test('소유 증거: 구글 계정 식별자가 세션과 일치하면 수용한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    google: { obfuscatedExternalAccountId: SESSION_UUID, expiredObfuscatedExternalAccountId: null },
  });

  assert.deepEqual(result, { kind: 'accepted', legacy: false });
});

test('소유 증거: 구글 계정 식별자가 어긋나면 거절한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    google: { obfuscatedExternalAccountId: OTHER_UUID, expiredObfuscatedExternalAccountId: null },
  });

  assert.deepEqual(result, { kind: 'rejected', reason: 'mismatch' });
});

test('소유 증거: 구글 계정 식별자 부재 시 만료 구매의 식별자로 대조해 수용한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    google: { obfuscatedExternalAccountId: null, expiredObfuscatedExternalAccountId: SESSION_UUID },
  });

  assert.deepEqual(result, { kind: 'accepted', legacy: false });
});

test('소유 증거: 구글 만료 구매 식별자가 어긋나면 거절한다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    google: { obfuscatedExternalAccountId: null, expiredObfuscatedExternalAccountId: OTHER_UUID },
  });

  assert.deepEqual(result, { kind: 'rejected', reason: 'mismatch' });
});

test('소유 증거: 구글 식별자가 양쪽 모두 없으면 레거시로 통과시킨다', () => {
  const result = validateIapRegistrationOwnership({
    sessionUuid: SESSION_UUID,
    google: { obfuscatedExternalAccountId: null, expiredObfuscatedExternalAccountId: null },
  });

  assert.deepEqual(result, { kind: 'accepted', legacy: true });
});
