import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import test from 'node:test';
import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { findPromotionConflict, isRevivalGated, reconcileWindowStart, selectGooglePredecessor } from './iap-sync-core.ts';
import type { EntitlementJudgmentRow } from './entitlement.ts';
import type { NormalizedIap } from './iap-normalize.ts';
import type { ConflictCandidate } from './iap-sync-core.ts';

const now = dayjs('2026-08-15T03:00:00.000Z');

const tracked = (over: Partial<Extract<NormalizedIap, { kind: 'tracked' }>> = {}): Extract<NormalizedIap, { kind: 'tracked' }> => ({
  kind: 'tracked',
  state: SubscriptionState.ACTIVE,
  planKey: 'pl0fl1map',
  periodStartsAt: now.subtract(10, 'days'),
  periodEndsAt: now.add(20, 'days'),
  intent: 'ON',
  successorTokens: [],
  acknowledgePending: false,
  productId: null,
  ...over,
});

const candidate = (over: Partial<ConflictCandidate> = {}): ConflictCandidate => ({
  id: 'sub_other',
  state: SubscriptionState.ACTIVE,
  planAvailability: PlanAvailability.BILLING_KEY,
  startsAt: now.subtract(60, 'days'),
  currentPeriodStartsAt: now.subtract(10, 'days'),
  currentPeriodEndsAt: now.add(20, 'days'),
  ...over,
});

test('복권 게이트: EXPIRED canonical 에 기간이 지난 유료 응답은 복권하지 않는다', () => {
  assert.equal(
    isRevivalGated(SubscriptionState.EXPIRED, tracked({ state: SubscriptionState.WILL_EXPIRE, periodEndsAt: now.subtract(1, 'day') }), now),
    true,
  );
});

test('복권 게이트: EXPIRED canonical 이라도 기간이 미래면 복권한다', () => {
  assert.equal(isRevivalGated(SubscriptionState.EXPIRED, tracked({ periodEndsAt: now.add(1, 'day') }), now), false);
});

test('복권 게이트: EXPIRED canonical 이라도 스토어 유예면 복권한다', () => {
  assert.equal(
    isRevivalGated(
      SubscriptionState.EXPIRED,
      tracked({ state: SubscriptionState.IN_GRACE_PERIOD, periodEndsAt: now.subtract(1, 'day') }),
      now,
    ),
    false,
  );
});

test('복권 게이트: canonical 이 EXPIRED 가 아니면 게이트가 없다', () => {
  assert.equal(isRevivalGated(SubscriptionState.ACTIVE, tracked({ periodEndsAt: now.subtract(1, 'day') }), now), false);
  assert.equal(isRevivalGated(undefined, tracked({ periodEndsAt: now.subtract(1, 'day') }), now), false);
});

test('구글 predecessor 선택: 승계 토큰 순서가 우선순위다', () => {
  const rows = [{ identifier: 'B' }, { identifier: 'A' }];
  assert.deepEqual(selectGooglePredecessor(['A', 'B'], rows), { identifier: 'A' });
  assert.deepEqual(selectGooglePredecessor(['B', 'A'], rows), { identifier: 'B' });
});

test('구글 predecessor 선택: 일치하는 행이 없으면 null', () => {
  assert.equal(selectGooglePredecessor(['A'], [{ identifier: 'B' }]), null);
  assert.equal(selectGooglePredecessor([], [{ identifier: 'B' }]), null);
});

const canonical = (over: Partial<EntitlementJudgmentRow> = {}): EntitlementJudgmentRow => ({
  state: SubscriptionState.EXPIRED,
  planAvailability: PlanAvailability.IN_APP_PURCHASE,
  startsAt: now.subtract(60, 'days'),
  currentPeriodStartsAt: now.subtract(40, 'days'),
  currentPeriodEndsAt: now.subtract(10, 'days'),
  ...over,
});

const conflict = (over: Partial<Parameters<typeof findPromotionConflict>[0]> = {}) =>
  findPromotionConflict({ candidates: [candidate()], canonical: canonical(), normalized: tracked(), now, ...over });

test('승격 충돌: 비-IAP ACTIVE·WILL_ACTIVATE 는 live 전이와 충돌한다', () => {
  assert.deepEqual(conflict(), candidate());
  assert.deepEqual(
    conflict({ candidates: [candidate({ state: SubscriptionState.WILL_ACTIVATE })] }),
    candidate({ state: SubscriptionState.WILL_ACTIVATE }),
  );
});

test('승격 충돌: 살아 있는 다른 IAP 계보는 live 전이와 충돌한다', () => {
  const other = candidate({ planAvailability: PlanAvailability.IN_APP_PURCHASE, state: SubscriptionState.WILL_EXPIRE });
  assert.deepEqual(conflict({ candidates: [other] }), other);
  assert.deepEqual(conflict({ candidates: [other], normalized: tracked({ state: SubscriptionState.WILL_EXPIRE }) }), other);
});

test('승격 충돌: 기간이 지난 다른 IAP 계보·비-IAP WILL_EXPIRE 는 충돌이 아니다', () => {
  assert.equal(
    conflict({
      candidates: [
        candidate({
          planAvailability: PlanAvailability.IN_APP_PURCHASE,
          state: SubscriptionState.WILL_EXPIRE,
          currentPeriodEndsAt: now.subtract(1, 'minute'),
        }),
      ],
    }),
    null,
  );
  assert.equal(conflict({ candidates: [candidate({ state: SubscriptionState.WILL_EXPIRE })] }), null);
});

test('승격 충돌: 이미 살아 있는 계보의 통상 갱신은 다른 IAP 계보와 충돌하지 않는다', () => {
  const other = candidate({ planAvailability: PlanAvailability.IN_APP_PURCHASE, state: SubscriptionState.WILL_EXPIRE });
  assert.equal(
    conflict({ candidates: [other], canonical: canonical({ state: SubscriptionState.ACTIVE, currentPeriodEndsAt: now.add(10, 'days') }) }),
    null,
  );
});

test('승격 충돌: 다른 IAP 계보가 ACTIVE 면 계보가 살아 있어도 충돌이다', () => {
  const other = candidate({ planAvailability: PlanAvailability.IN_APP_PURCHASE, state: SubscriptionState.ACTIVE });
  assert.deepEqual(
    conflict({
      candidates: [other],
      canonical: canonical({ state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now.add(10, 'days') }),
    }),
    other,
  );
});

test('승격 충돌: 목표 ACTIVE 의 비-IAP 충돌은 계보가 이미 살아 있어도 검사한다 — ACTIVE 유니크 위반 방지', () => {
  assert.deepEqual(
    conflict({ canonical: canonical({ state: SubscriptionState.WILL_EXPIRE, currentPeriodEndsAt: now.add(10, 'days') }) }),
    candidate(),
  );
});

test('승격 충돌: 목표가 live 가 아니면 충돌이 없다', () => {
  assert.equal(conflict({ normalized: tracked({ state: SubscriptionState.WILL_EXPIRE, periodEndsAt: now.subtract(1, 'day') }) }), null);
  assert.equal(conflict({ normalized: { kind: 'expired', observed: null } }), null);
});

test('승격 충돌: 비매칭 행을 건너뛰고 뒤의 매칭 행을 찾는다', () => {
  const hit = candidate({ id: 'sub_hit' });
  assert.deepEqual(conflict({ candidates: [candidate({ state: SubscriptionState.WILL_EXPIRE }), hit] }), hit);
});

test('재조정 창: 종료 확정 후 90일이 창의 시작이다', () => {
  assert.equal(reconcileWindowStart(now).toISOString(), now.subtract(90, 'days').toISOString());
});
