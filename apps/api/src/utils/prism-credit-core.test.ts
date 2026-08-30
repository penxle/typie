import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import test from 'node:test';
import dayjs from 'dayjs';
import {
  clampExpiringMilli,
  computeTrialExpiresAt,
  computeTrialRemainder,
  invertCharge,
  MILLI_PER_CREDIT,
  splitCharge,
  toDisplayCredits,
  toMilli,
  TRIAL_CREDIT_AMOUNT,
  TRIAL_EXPIRY_DAYS,
  validateEntry,
} from './prism-credit-core.ts';

test('splitCharge: 무상 잔액이 충분하면 전액 무상에서 뺀다', () => {
  assert.deepEqual(splitCharge({ free: 5000, amount: 3000 }), { paidDelta: 0, freeDelta: -3000 });
});

test('splitCharge: 무상이 모자라면 나머지를 유상에서 뺀다', () => {
  assert.deepEqual(splitCharge({ free: 1000, amount: 3000 }), { paidDelta: -2000, freeDelta: -1000 });
});

test('splitCharge: 무상이 0이면 전액 유상', () => {
  assert.deepEqual(splitCharge({ free: 0, amount: 3000 }), { paidDelta: -3000, freeDelta: 0 });
});

test('splitCharge: 무상 잔액이 음수로 들어와도 무상에서 더 빼지 않는다', () => {
  assert.deepEqual(splitCharge({ free: -400, amount: 3000 }), { paidDelta: -3000, freeDelta: 0 });
});

test('splitCharge: amount 0은 0/0 엔트리', () => {
  assert.deepEqual(splitCharge({ free: 5000, amount: 0 }), { paidDelta: 0, freeDelta: 0 });
});

test('splitCharge: 음수·비정수 amount는 거부한다', () => {
  assert.throws(() => splitCharge({ free: 0, amount: -1 }));
  assert.throws(() => splitCharge({ free: 0, amount: 1.5 }));
});

test('splitCharge: 무작위 입력에서 free 결과는 결코 음수가 되지 않는다', () => {
  let seed = 20_260_826;
  const next = () => {
    seed = (seed * 1_103_515_245 + 12_345) % 2_147_483_648;
    return seed;
  };
  for (let i = 0; i < 2000; i++) {
    const free = (next() % 200_000) - 100_000;
    const amount = next() % 150_000;
    const { paidDelta, freeDelta } = splitCharge({ free, amount });
    assert.ok(free + freeDelta >= Math.min(free, 0));
    assert.ok(freeDelta <= 0 && paidDelta <= 0);
    assert.equal(paidDelta + freeDelta, 0 - amount);
  }
});

test('invertCharge: 반전의 반전은 항등', () => {
  const delta = { paidDelta: -2000, freeDelta: -1000 };
  assert.deepEqual(invertCharge(delta), { paidDelta: 2000, freeDelta: 1000 });
  assert.deepEqual(invertCharge(invertCharge(delta)), delta);
});

test('invertCharge: 반전 결과도 0을 정본으로 낸다', () => {
  assert.deepEqual(invertCharge(splitCharge({ free: 5000, amount: 3000 })), { paidDelta: 0, freeDelta: 3000 });
  assert.deepEqual(invertCharge({ paidDelta: 0, freeDelta: 0 }), { paidDelta: 0, freeDelta: 0 });
});

test('validateEntry: GRANT/TRIAL은 무상 양수·유상 0만 허용', () => {
  validateEntry('GRANT', { paidDelta: 0, freeDelta: 1000 });
  validateEntry('TRIAL', { paidDelta: 0, freeDelta: 1 });
  assert.throws(() => validateEntry('GRANT', { paidDelta: 0, freeDelta: 0 }));
  assert.throws(() => validateEntry('GRANT', { paidDelta: 1000, freeDelta: 0 }));
  assert.throws(() => validateEntry('TRIAL', { paidDelta: 0, freeDelta: -1 }));
});

test('validateEntry: 차감 2종은 두 delta 모두 0 이하', () => {
  validateEntry('REVIEW_CHARGE', { paidDelta: -1, freeDelta: 0 });
  validateEntry('CHAT_CHARGE', { paidDelta: 0, freeDelta: 0 });
  assert.throws(() => validateEntry('REVIEW_CHARGE', { paidDelta: 1, freeDelta: 0 }));
  assert.throws(() => validateEntry('CHAT_CHARGE', { paidDelta: 0, freeDelta: 1 }));
});

test('validateEntry: REVIEW_REFUND는 두 delta 모두 0 이상', () => {
  validateEntry('REVIEW_REFUND', { paidDelta: 2000, freeDelta: 0 });
  assert.throws(() => validateEntry('REVIEW_REFUND', { paidDelta: -1, freeDelta: 0 }));
});

test('validateEntry: ADJUSTMENT는 부호 자유, 정수만', () => {
  validateEntry('ADJUSTMENT', { paidDelta: -5, freeDelta: 7 });
  assert.throws(() => validateEntry('ADJUSTMENT', { paidDelta: 0.5, freeDelta: 0 }));
});

test('toDisplayCredits: 0에서 멀어지는 올림', () => {
  assert.equal(toDisplayCredits(0), 0);
  assert.equal(toDisplayCredits(1), 1);
  assert.equal(toDisplayCredits(999), 1);
  assert.equal(toDisplayCredits(1000), 1);
  assert.equal(toDisplayCredits(1001), 2);
  assert.equal(toDisplayCredits(-1), -1);
  assert.equal(toDisplayCredits(-999), -1);
  assert.equal(toDisplayCredits(-1000), -1);
  assert.equal(toDisplayCredits(-1001), -2);
  assert.ok(Object.is(toDisplayCredits(0), 0));
});

test('toMilli: 정수 크레딧을 밀리로', () => {
  assert.equal(toMilli(3), 3 * MILLI_PER_CREDIT);
  assert.equal(toMilli(-2), -2000);
  assert.throws(() => toMilli(1.5));
});

test('validateEntry: PURCHASE는 paid>0·free=0, BONUS는 free>0·paid=0, REFUND_OUT은 둘 다 ≤0이고 둘 다 0은 아님', () => {
  validateEntry('PURCHASE', { paidDelta: 100_000, freeDelta: 0 });
  assert.throws(() => validateEntry('PURCHASE', { paidDelta: 100_000, freeDelta: 1 }), /PURCHASE/);
  assert.throws(() => validateEntry('PURCHASE', { paidDelta: 0, freeDelta: 0 }), /PURCHASE/);

  validateEntry('BONUS', { paidDelta: 0, freeDelta: 30_000 });
  assert.throws(() => validateEntry('BONUS', { paidDelta: 1, freeDelta: 30_000 }), /BONUS/);

  validateEntry('REFUND_OUT', { paidDelta: -100_000, freeDelta: 0 });
  validateEntry('REFUND_OUT', { paidDelta: -100_000, freeDelta: -30_000 });
  assert.throws(() => validateEntry('REFUND_OUT', { paidDelta: 0, freeDelta: 0 }), /REFUND_OUT/);
  assert.throws(() => validateEntry('REFUND_OUT', { paidDelta: 1, freeDelta: 0 }), /REFUND_OUT/);
  assert.throws(() => validateEntry('REFUND_OUT', { paidDelta: 0, freeDelta: 1 }), /REFUND_OUT/);
  assert.throws(() => validateEntry('BONUS', { paidDelta: 0, freeDelta: 0 }), /BONUS/);
  assert.throws(() => validateEntry('BONUS', { paidDelta: 0, freeDelta: -1 }), /BONUS/);
});

test('validateEntry: EXPIRE는 무상 음수·유상 0만 허용', () => {
  validateEntry('EXPIRE', { paidDelta: 0, freeDelta: -1000 });
  assert.throws(() => validateEntry('EXPIRE', { paidDelta: 0, freeDelta: 0 }));
  assert.throws(() => validateEntry('EXPIRE', { paidDelta: 0, freeDelta: 1000 }));
  assert.throws(() => validateEntry('EXPIRE', { paidDelta: -1000, freeDelta: -1000 }));
});

test('computeTrialRemainder: 소진한 만큼만 줄어든다', () => {
  assert.equal(computeTrialRemainder({ granted: 300_000, consumedNet: -250_000 }), 50_000);
});

test('computeTrialRemainder: 반환이 소진을 상쇄하면 전액 복원', () => {
  assert.equal(computeTrialRemainder({ granted: 300_000, consumedNet: 0 }), 300_000);
});

test('computeTrialRemainder: GRANT 병존 — 소진은 TRIAL부터 깎는다', () => {
  assert.equal(computeTrialRemainder({ granted: 600_000, consumedNet: -300_000 }), 300_000);
});

test('computeTrialRemainder: 백필 순서 역전 — TRIAL 이전 소진은 세지 않는다', () => {
  assert.equal(computeTrialRemainder({ granted: 600_000, consumedNet: 0 }), 600_000);
});

test('computeTrialRemainder: 지급액을 넘겨 소진하면 0에서 멈춘다', () => {
  assert.equal(computeTrialRemainder({ granted: 600_000, consumedNet: -700_000 }), 0);
});

test('computeTrialRemainder: 순합이 양수여도 지급액을 넘지 않는다', () => {
  assert.equal(computeTrialRemainder({ granted: 300_000, consumedNet: 120_000 }), 300_000);
});

test('computeTrialExpiresAt: KST 자정 배타적 경계로 30일', () => {
  const at = computeTrialExpiresAt(dayjs('2026-09-01T15:30:00+09:00'));
  assert.equal(at.toISOString(), dayjs('2026-10-01T00:00:00+09:00').toISOString());
});

test('computeTrialExpiresAt: KST 자정 직전 지급도 그날 기준으로 끊는다', () => {
  const at = computeTrialExpiresAt(dayjs('2026-09-01T23:59:59+09:00'));
  assert.equal(at.toISOString(), dayjs('2026-10-01T00:00:00+09:00').toISOString());
});

test('computeTrialExpiresAt: UTC 입력도 KST 날짜로 끊는다', () => {
  const at = computeTrialExpiresAt(dayjs.utc('2026-08-31T16:00:00Z'));
  assert.equal(at.toISOString(), dayjs('2026-10-01T00:00:00+09:00').toISOString());
});

test('computeTrialExpiresAt: UTC 입력이 KST 자정 직전이어도 그날 기준으로 끊는다', () => {
  const at = computeTrialExpiresAt(dayjs.utc('2026-09-01T14:59:59Z'));
  assert.equal(at.toISOString(), dayjs('2026-10-01T00:00:00+09:00').toISOString());
});

test('clampExpiringMilli: 보유 잔액을 넘지 않는다', () => {
  assert.equal(clampExpiringMilli({ remainder: 600_000, total: 550_000 }), 550_000);
});

test('clampExpiringMilli: 보유가 넉넉하면 잔량 그대로', () => {
  assert.equal(clampExpiringMilli({ remainder: 300_000, total: 400_000 }), 300_000);
});

test('clampExpiringMilli: 보유가 음수면 0', () => {
  assert.equal(clampExpiringMilli({ remainder: 300_000, total: -50_000 }), 0);
});

test('상수: 지급량 300 크레딧, 기간 30일', () => {
  assert.equal(TRIAL_CREDIT_AMOUNT, 300);
  assert.equal(TRIAL_EXPIRY_DAYS, 30);
  assert.equal(toMilli(TRIAL_CREDIT_AMOUNT), 300 * MILLI_PER_CREDIT);
});
