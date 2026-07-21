import assert from 'node:assert/strict';
import test from 'node:test';
import { PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { resolveEnrollAction } from './subscription-enroll.ts';
import type { EnrollSubscriptionRow } from './subscription-enroll.ts';

const now = dayjs('2026-07-22T12:00:00.000Z');
const future = now.add(7, 'days');
const past = now.subtract(1, 'hour');

const row = (state: SubscriptionState, planAvailability: PlanAvailability, expiresAt: dayjs.Dayjs): EnrollSubscriptionRow => ({
  state,
  planAvailability,
  expiresAt,
});

test('구독 이력이 없으면 즉시 결제', () => {
  assert.deepEqual(resolveEnrollAction([], now), { kind: 'immediate' });
});

test('진행 중 트라이얼은 만료 시각 시작 예약', () => {
  const action = resolveEnrollAction([row(SubscriptionState.WILL_EXPIRE, PlanAvailability.TRIAL, future)], now);
  assert.equal(action.kind, 'schedule');
  assert.ok(action.kind === 'schedule' && action.startsAt.isSame(future));
});

test('만료 시각이 지난 트라이얼(해지 확정 잡 지연)은 즉시 결제', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.WILL_EXPIRE, PlanAvailability.TRIAL, past)], now), { kind: 'immediate' });
});

test('시간상 만료된 해지 예정 빌링키 구독은 즉시 결제 — 잡 지연이 재가입을 막지 않는다', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.WILL_EXPIRE, PlanAvailability.BILLING_KEY, past)], now), {
    kind: 'immediate',
  });
});

test('유효 빌링키 구독이 있으면 거부', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.ACTIVE, PlanAvailability.BILLING_KEY, future)], now), { kind: 'reject' });
});

test('유예 중 구독이 있으면 거부(시간 경과와 무관)', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.IN_GRACE_PERIOD, PlanAvailability.BILLING_KEY, past)], now), {
    kind: 'reject',
  });
});

test('해지 예정 빌링키 구독이 만료 전이면 거부', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.WILL_EXPIRE, PlanAvailability.BILLING_KEY, future)], now), {
    kind: 'reject',
  });
});

test('IAP 구독이 있으면 거부', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.ACTIVE, PlanAvailability.IN_APP_PURCHASE, future)], now), {
    kind: 'reject',
  });
});

test('전환 공존 창(새 ACTIVE + 옛 트라이얼)은 거부 — 단일 행 대표 오판 방지', () => {
  const action = resolveEnrollAction(
    [row(SubscriptionState.WILL_EXPIRE, PlanAvailability.TRIAL, past), row(SubscriptionState.ACTIVE, PlanAvailability.BILLING_KEY, future)],
    now,
  );
  assert.deepEqual(action, { kind: 'reject' });
});

test('기존 예약이 있어도 진행 중 트라이얼이면 예약(교체)', () => {
  const action = resolveEnrollAction(
    [
      row(SubscriptionState.WILL_EXPIRE, PlanAvailability.TRIAL, future),
      row(SubscriptionState.WILL_ACTIVATE, PlanAvailability.BILLING_KEY, future.add(30, 'days')),
    ],
    now,
  );
  assert.equal(action.kind, 'schedule');
});

test('유령 예약만 남은 만료 유저는 즉시 결제', () => {
  assert.deepEqual(resolveEnrollAction([row(SubscriptionState.WILL_ACTIVATE, PlanAvailability.BILLING_KEY, future)], now), {
    kind: 'immediate',
  });
});
