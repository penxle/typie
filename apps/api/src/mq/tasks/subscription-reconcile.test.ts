import assert from 'node:assert/strict';
import test from 'node:test';
import { SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { resolveReconcileAction } from './subscription-reconcile.ts';
import type { ReconcileAction, ReconcileStoreState } from './subscription-reconcile.ts';

const now = dayjs('2026-07-01T00:00:00.000Z');
const recentlyExpired = now.subtract(1, 'hour'); // 만료 경과(하루 미만)
const longExpired = now.subtract(2, 'days'); // 만료 경과(하루 초과)
const future = now.add(30, 'days');
const storeLater = now.add(30, 'days'); // 스토어가 알려준 더 나중 만료일
const storeMuchLater = now.add(60, 'days'); // 로컬(미래)보다도 더 나중인 스토어 만료일

const resolve = (params: {
  storeState: ReconcileStoreState;
  storeExpiresAt?: dayjs.Dayjs | null;
  currentState: SubscriptionState;
  currentExpiresAt: dayjs.Dayjs;
}): ReconcileAction =>
  resolveReconcileAction({
    storeState: params.storeState,
    storeExpiresAt: params.storeExpiresAt ?? null,
    currentState: params.currentState,
    currentExpiresAt: params.currentExpiresAt,
    now,
  });

// active: 유실된 갱신 복구
test('active + stale ACTIVE with a later store expiry recovers', () => {
  const action = resolve({
    storeState: 'active',
    storeExpiresAt: storeLater,
    currentState: SubscriptionState.ACTIVE,
    currentExpiresAt: recentlyExpired,
  });
  assert.equal(action.type, 'recover');
  assert.ok(action.type === 'recover' && action.expiresAt.isSame(storeLater));
});

test('active recovers a WILL_EXPIRE row (cancel then re-enable)', () => {
  assert.equal(
    resolve({
      storeState: 'active',
      storeExpiresAt: storeLater,
      currentState: SubscriptionState.WILL_EXPIRE,
      currentExpiresAt: longExpired,
    }).type,
    'recover',
  );
});

test('active advances a not-yet-expired row when the store confirms a strictly later expiry (missed renewal, no 24h lockout)', () => {
  const action = resolve({
    storeState: 'active',
    storeExpiresAt: storeMuchLater,
    currentState: SubscriptionState.ACTIVE,
    currentExpiresAt: future,
  });
  assert.equal(action.type, 'recover');
  assert.ok(action.type === 'recover' && action.expiresAt.isSame(storeMuchLater));
});

test('active recovers an IN_GRACE_PERIOD row', () => {
  assert.equal(
    resolve({
      storeState: 'active',
      storeExpiresAt: storeLater,
      currentState: SubscriptionState.IN_GRACE_PERIOD,
      currentExpiresAt: longExpired,
    }).type,
    'recover',
  );
});

test('active never resurrects an EXPIRED (refunded/revoked) row', () => {
  assert.deepEqual(
    resolve({ storeState: 'active', storeExpiresAt: storeLater, currentState: SubscriptionState.EXPIRED, currentExpiresAt: longExpired }),
    { type: 'none' },
  );
});

test('active is a no-op for a healthy row whose expiry is still in the future', () => {
  assert.deepEqual(
    resolve({ storeState: 'active', storeExpiresAt: future, currentState: SubscriptionState.ACTIVE, currentExpiresAt: future }),
    { type: 'none' },
  );
});

test('active does not shorten expiry when the store expiry is not later', () => {
  assert.deepEqual(
    resolve({
      storeState: 'active',
      storeExpiresAt: recentlyExpired,
      currentState: SubscriptionState.ACTIVE,
      currentExpiresAt: recentlyExpired,
    }),
    { type: 'none' },
  );
});

test('active is a no-op without a store expiry', () => {
  assert.deepEqual(
    resolve({ storeState: 'active', storeExpiresAt: null, currentState: SubscriptionState.ACTIVE, currentExpiresAt: longExpired }),
    { type: 'none' },
  );
});

// grace: 유예 접근 복원
test('grace restores IN_GRACE_PERIOD from a stale ACTIVE row', () => {
  assert.deepEqual(resolve({ storeState: 'grace', currentState: SubscriptionState.ACTIVE, currentExpiresAt: recentlyExpired }), {
    type: 'grace',
  });
});

test('grace restores IN_GRACE_PERIOD from a WILL_EXPIRE row', () => {
  assert.deepEqual(resolve({ storeState: 'grace', currentState: SubscriptionState.WILL_EXPIRE, currentExpiresAt: recentlyExpired }), {
    type: 'grace',
  });
});

test('grace is a no-op when the row is already IN_GRACE_PERIOD', () => {
  assert.deepEqual(resolve({ storeState: 'grace', currentState: SubscriptionState.IN_GRACE_PERIOD, currentExpiresAt: recentlyExpired }), {
    type: 'none',
  });
});

test('grace never resurrects an EXPIRED row', () => {
  assert.deepEqual(resolve({ storeState: 'grace', currentState: SubscriptionState.EXPIRED, currentExpiresAt: longExpired }), {
    type: 'none',
  });
});

test('grace is a no-op while the paid period is still in the future', () => {
  assert.deepEqual(resolve({ storeState: 'grace', currentState: SubscriptionState.ACTIVE, currentExpiresAt: future }), { type: 'none' });
});

// suspended: 보류·재청구·일시중지 (비권한이나 복구 가능)
test('suspended moves a stale ACTIVE row to WILL_EXPIRE', () => {
  assert.deepEqual(resolve({ storeState: 'suspended', currentState: SubscriptionState.ACTIVE, currentExpiresAt: recentlyExpired }), {
    type: 'suspend',
  });
});

test('suspended moves an IN_GRACE_PERIOD row to WILL_EXPIRE', () => {
  assert.deepEqual(
    resolve({ storeState: 'suspended', currentState: SubscriptionState.IN_GRACE_PERIOD, currentExpiresAt: recentlyExpired }),
    {
      type: 'suspend',
    },
  );
});

test('suspended is a no-op when the row is already WILL_EXPIRE', () => {
  assert.deepEqual(resolve({ storeState: 'suspended', currentState: SubscriptionState.WILL_EXPIRE, currentExpiresAt: recentlyExpired }), {
    type: 'none',
  });
});

test('suspended never resurrects an EXPIRED row', () => {
  assert.deepEqual(resolve({ storeState: 'suspended', currentState: SubscriptionState.EXPIRED, currentExpiresAt: longExpired }), {
    type: 'none',
  });
});

test('suspended is a no-op while the paid period is still in the future (pause scheduled, not in effect)', () => {
  assert.deepEqual(resolve({ storeState: 'suspended', currentState: SubscriptionState.ACTIVE, currentExpiresAt: future }), {
    type: 'none',
  });
});

// expired: 일반 만료 (하루 여유)
test('expired expires a row whose local expiry is more than a day old', () => {
  assert.deepEqual(resolve({ storeState: 'expired', currentState: SubscriptionState.ACTIVE, currentExpiresAt: longExpired }), {
    type: 'expire',
  });
});

test('expired waits when the local expiry is only recently past (buffer against store API lag)', () => {
  assert.deepEqual(resolve({ storeState: 'expired', currentState: SubscriptionState.ACTIVE, currentExpiresAt: recentlyExpired }), {
    type: 'none',
  });
});

test('expired revokes immediately when the store ended a still-future local period (revocation/chargeback)', () => {
  assert.deepEqual(resolve({ storeState: 'expired', currentState: SubscriptionState.ACTIVE, currentExpiresAt: future }), {
    type: 'expire',
  });
});

test('expired expires a WILL_EXPIRE row past the buffer', () => {
  assert.deepEqual(resolve({ storeState: 'expired', currentState: SubscriptionState.WILL_EXPIRE, currentExpiresAt: longExpired }), {
    type: 'expire',
  });
});

test('expired is a no-op for a row that is already EXPIRED', () => {
  assert.deepEqual(resolve({ storeState: 'expired', currentState: SubscriptionState.EXPIRED, currentExpiresAt: longExpired }), {
    type: 'none',
  });
});

// revoked: 환불·철회 (즉시, 만료일 무관)
test('revoked expires a mid-term row immediately even with a future expiry', () => {
  assert.deepEqual(resolve({ storeState: 'revoked', currentState: SubscriptionState.ACTIVE, currentExpiresAt: future }), {
    type: 'expire',
  });
});

test('revoked expires an IN_GRACE_PERIOD row immediately', () => {
  assert.deepEqual(resolve({ storeState: 'revoked', currentState: SubscriptionState.IN_GRACE_PERIOD, currentExpiresAt: recentlyExpired }), {
    type: 'expire',
  });
});

test('revoked is a no-op for a row that is already EXPIRED', () => {
  assert.deepEqual(resolve({ storeState: 'revoked', currentState: SubscriptionState.EXPIRED, currentExpiresAt: longExpired }), {
    type: 'none',
  });
});

// unknown: 조회 실패 — 아무 것도 하지 않음
test('unknown is always a no-op', () => {
  assert.deepEqual(resolve({ storeState: 'unknown', currentState: SubscriptionState.ACTIVE, currentExpiresAt: longExpired }), {
    type: 'none',
  });
});
