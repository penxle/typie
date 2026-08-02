import assert from 'node:assert/strict';
import test from 'node:test';
import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import {
  deriveExpiresAtShim,
  deriveGraceDeadline,
  isSubscriptionEntitled,
  resolveUserEntitlement,
  selectRepresentativeSubscription,
} from './entitlement.ts';
import type { EntitlementSubscriptionRow } from './entitlement.ts';

const now = dayjs('2026-08-15T03:00:00.000Z');

const row = (over: Partial<EntitlementSubscriptionRow>): EntitlementSubscriptionRow => ({
  id: 'sub_1',
  state: SubscriptionState.ACTIVE,
  planAvailability: PlanAvailability.BILLING_KEY,
  startsAt: now.subtract(60, 'days'),
  currentPeriodStartsAt: now.subtract(20, 'days'),
  currentPeriodEndsAt: now.add(10, 'days'),
  createdAt: now.subtract(60, 'days'),
  ...over,
});

test('ACTIVE는 기간이 지나도 허용 — 시각 비교 없음', () => {
  assert.equal(isSubscriptionEntitled(row({ currentPeriodEndsAt: now.subtract(9, 'hours') }), now), true);
});

test('WILL_EXPIRE는 기간 내에만 허용', () => {
  assert.equal(isSubscriptionEntitled(row({ state: SubscriptionState.WILL_EXPIRE }), now), true);
  assert.equal(isSubscriptionEntitled(row({ state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now }), now), false);
});

test('정기 갱신 유예 마감 = 주기 종료 + 7일 (주기는 이전 주기에 머묾)', () => {
  const r = row({ state: SubscriptionState.IN_GRACE_PERIOD, currentPeriodEndsAt: now.subtract(2, 'days') });
  assert.ok(deriveGraceDeadline(r, now).isSame(now.subtract(2, 'days').add(7, 'days')));
});

test('전환 유예 마감 = 선설정된 주기 시작 + 7일', () => {
  const r = row({
    state: SubscriptionState.IN_GRACE_PERIOD,
    currentPeriodStartsAt: now.subtract(1, 'day'),
    currentPeriodEndsAt: now.add(29, 'days'),
  });
  assert.ok(deriveGraceDeadline(r, now).isSame(now.subtract(1, 'day').add(7, 'days')));
});

test('IAP 유예 마감 = 주기 종료 + 31일 백스톱', () => {
  const r = row({
    state: SubscriptionState.IN_GRACE_PERIOD,
    planAvailability: PlanAvailability.IN_APP_PURCHASE,
    currentPeriodEndsAt: now.subtract(3, 'days'),
  });
  assert.ok(deriveGraceDeadline(r, now).isSame(now.subtract(3, 'days').add(31, 'days')));
});

test('WILL_ACTIVATE는 시작 경과면 허용(전환 지연 fail-open), 미래면 회수', () => {
  assert.equal(isSubscriptionEntitled(row({ state: SubscriptionState.WILL_ACTIVATE, startsAt: now.subtract(1, 'minute') }), now), true);
  assert.equal(isSubscriptionEntitled(row({ state: SubscriptionState.WILL_ACTIVATE, startsAt: now.add(1, 'minute') }), now), false);
});

test('entitledUntil: ACTIVE가 있으면 null — 시각으로 안 뒤집힘', () => {
  const r = resolveUserEntitlement([row({}), row({ id: 'sub_2', state: SubscriptionState.WILL_EXPIRE })], now);
  assert.deepEqual([r.entitled, r.entitledUntil], [true, null]);
});

test('entitledUntil: timed 행만 있으면 유효 마감의 최대값', () => {
  const r = resolveUserEntitlement(
    [
      row({ state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now.add(3, 'days') }),
      row({ id: 'sub_2', state: SubscriptionState.IN_GRACE_PERIOD, currentPeriodEndsAt: now.subtract(1, 'day') }),
    ],
    now,
  );
  assert.ok(r.entitledUntil?.isSame(now.subtract(1, 'day').add(7, 'days')));
});

// --- 브리프 추가 케이스 ---

test('EXPIRED는 무조건 회수', () => {
  assert.equal(isSubscriptionEntitled(row({ state: SubscriptionState.EXPIRED }), now), false);
});

test('유예 마감이 now와 정확히 같으면 회수(엄격 부등호, 등호는 불허)', () => {
  const r = row({ state: SubscriptionState.IN_GRACE_PERIOD, currentPeriodEndsAt: now.subtract(7, 'days') });
  assert.ok(deriveGraceDeadline(r, now).isSame(now));
  assert.equal(isSubscriptionEntitled(r, now), false);
});

test('유예 파생 후보 없음(주기 컬럼이 둘 다 미래, 전제 밖 방어) = 주기 시작 + 7일', () => {
  const r = row({
    state: SubscriptionState.IN_GRACE_PERIOD,
    currentPeriodStartsAt: now.add(1, 'day'),
    currentPeriodEndsAt: now.add(31, 'days'),
  });
  assert.ok(deriveGraceDeadline(r, now).isSame(now.add(1, 'day').add(7, 'days')));
});

test('entitled 행이 0개면 {false, null} — 빈 배열', () => {
  const r = resolveUserEntitlement([], now);
  assert.deepEqual([r.entitled, r.entitledUntil], [false, null]);
});

test('entitled 행이 0개면 {false, null} — 전부 EXPIRED', () => {
  const r = resolveUserEntitlement(
    [row({ state: SubscriptionState.EXPIRED }), row({ id: 'sub_2', state: SubscriptionState.EXPIRED })],
    now,
  );
  assert.deepEqual([r.entitled, r.entitledUntil], [false, null]);
});

test('대표 선택 규칙 1: 권한 있는 WILL_EXPIRE가 권한 없는(마감 경과) IN_GRACE_PERIOD보다 우선', () => {
  const entitledWillExpire = row({ id: 'sub_we', state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now.add(1, 'day') });
  const expiredGrace = row({
    id: 'sub_grace',
    state: SubscriptionState.IN_GRACE_PERIOD,
    currentPeriodStartsAt: now.subtract(60, 'days'),
    currentPeriodEndsAt: now.subtract(30, 'days'),
    createdAt: now.subtract(5, 'days'),
  });
  const rep = selectRepresentativeSubscription([entitledWillExpire, expiredGrace], now);
  assert.equal(rep?.id, 'sub_we');
});

test('대표 선택 규칙 2: ACTIVE가 더 최신인 WILL_EXPIRE보다 우선', () => {
  const active = row({ id: 'sub_active', createdAt: now.subtract(60, 'days') });
  const newerWillExpire = row({
    id: 'sub_we',
    state: SubscriptionState.WILL_EXPIRE,
    currentPeriodEndsAt: now.add(5, 'days'),
    createdAt: now.subtract(1, 'day'),
  });
  const rep = selectRepresentativeSubscription([active, newerWillExpire], now);
  assert.equal(rep?.id, 'sub_active');
});

test('대표 선택 규칙 3: 동급이면 최신순', () => {
  const older = row({ id: 'sub_old', createdAt: now.subtract(60, 'days') });
  const newer = row({ id: 'sub_new', createdAt: now.subtract(5, 'days') });
  const rep = selectRepresentativeSubscription([older, newer], now);
  assert.equal(rep?.id, 'sub_new');
});

test('대표 선택: 시작 경과 예약만 있는 유저는 그 예약 행이 대표', () => {
  const reserved = row({ id: 'sub_reserved', state: SubscriptionState.WILL_ACTIVATE, startsAt: now.subtract(1, 'minute') });
  const rep = selectRepresentativeSubscription([reserved], now);
  assert.equal(rep?.id, 'sub_reserved');
});

test('대표 선택: 시작 전 예약은 후보에서 제외 — 후보 없으면 null', () => {
  const notYetStarted = row({ id: 'sub_future', state: SubscriptionState.WILL_ACTIVATE, startsAt: now.add(1, 'minute') });
  const rep = selectRepresentativeSubscription([notYetStarted], now);
  assert.equal(rep, null);
});

test('shim: ACTIVE + 빌링키 = 주기 종료 + 48시간', () => {
  const r = row({});
  assert.ok(deriveExpiresAtShim(r, now).isSame(r.currentPeriodEndsAt.add(48, 'hours')));
});

test('shim: ACTIVE + 그 외 채널 = 주기 종료 그대로', () => {
  const r = row({ planAvailability: PlanAvailability.IN_APP_PURCHASE });
  assert.ok(deriveExpiresAtShim(r, now).isSame(r.currentPeriodEndsAt));
});

test('shim: WILL_EXPIRE는 전 채널 공통 주기 종료', () => {
  const r = row({ state: SubscriptionState.WILL_EXPIRE, planAvailability: PlanAvailability.IN_APP_PURCHASE });
  assert.ok(deriveExpiresAtShim(r, now).isSame(r.currentPeriodEndsAt));
});

test('shim: IN_GRACE_PERIOD 비IAP는 유예 마감 역산(-7일) = 미결제 주기 시작', () => {
  const r = row({ state: SubscriptionState.IN_GRACE_PERIOD, currentPeriodEndsAt: now.subtract(2, 'days') });
  assert.ok(deriveExpiresAtShim(r, now).isSame(deriveGraceDeadline(r, now).subtract(7, 'days')));
});

test('shim: IN_GRACE_PERIOD IAP는 주기 종료 그대로(구 앱이 31일 자체 가산)', () => {
  const r = row({
    state: SubscriptionState.IN_GRACE_PERIOD,
    planAvailability: PlanAvailability.IN_APP_PURCHASE,
    currentPeriodEndsAt: now.subtract(2, 'days'),
  });
  assert.ok(deriveExpiresAtShim(r, now).isSame(r.currentPeriodEndsAt));
});

test('shim: EXPIRED는 주기 종료와 now 중 이른 쪽 — 환불로 주기 종료가 미래에 남아도 now로 clip', () => {
  const r = row({ state: SubscriptionState.EXPIRED, currentPeriodEndsAt: now.add(3, 'days') });
  assert.ok(deriveExpiresAtShim(r, now).isSame(now));
});

test('shim: WILL_ACTIVATE 시작 전은 시작 시각과 now 중 이른 쪽 — now를 넘지 않게 clip', () => {
  const r = row({ state: SubscriptionState.WILL_ACTIVATE, startsAt: now.add(3, 'days') });
  const shim = deriveExpiresAtShim(r, now);
  assert.ok(shim.isSame(now));
  assert.ok(!shim.isAfter(now));
});

test('shim: WILL_ACTIVATE 시작 경과는 시작 시각 + 48시간(fail-open)', () => {
  const r = row({ state: SubscriptionState.WILL_ACTIVATE, startsAt: now.subtract(1, 'minute') });
  assert.ok(deriveExpiresAtShim(r, now).isSame(r.startsAt.add(48, 'hours')));
});
