import assert from 'node:assert/strict';
import test from 'node:test';
import { classifyAlreadyPaidRecovery, splitBillingAmount } from './payment-core.ts';

test('classifyAlreadyPaidRecovery: PAID 조회 금액과 일치하는 시도 레코드가 있으면 그 레코드를 증거로 성공 확정한다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'paid', amount: 4900 },
    attemptRecords: [{ billingAmount: 4900, creditAmount: 1000 }],
  });

  assert.deepEqual(classification, { kind: 'finalize', evidence: { billingAmount: 4900, creditAmount: 1000 } });
});

test('classifyAlreadyPaidRecovery: 일치 레코드가 최신이 아니어도 찾는다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'paid', amount: 3900 },
    attemptRecords: [
      { billingAmount: 4900, creditAmount: 0 },
      { billingAmount: 3900, creditAmount: 1000 },
    ],
  });

  assert.deepEqual(classification, { kind: 'finalize', evidence: { billingAmount: 3900, creditAmount: 1000 } });
});

test('classifyAlreadyPaidRecovery: 일치 레코드가 여럿이면 최신 것을 증거로 쓴다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'paid', amount: 4900 },
    attemptRecords: [
      { billingAmount: 4900, creditAmount: 0 },
      { billingAmount: 4900, creditAmount: 2900 },
    ],
  });

  assert.deepEqual(classification, { kind: 'finalize', evidence: { billingAmount: 4900, creditAmount: 0 } });
});

test('classifyAlreadyPaidRecovery: PAID인데 시도 레코드가 0건이면 증거 부재로 알람한다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'paid', amount: 4900 },
    attemptRecords: [],
  });

  assert.deepEqual(classification, { kind: 'alert', id: 'already-paid-without-evidence' });
});

test('classifyAlreadyPaidRecovery: PAID인데 어느 시도 레코드와도 금액이 다르면 불일치로 알람한다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'paid', amount: 4900 },
    attemptRecords: [
      { billingAmount: 3900, creditAmount: 1000 },
      { billingAmount: 0, creditAmount: 4900 },
    ],
  });

  assert.deepEqual(classification, { kind: 'alert', id: 'already-paid-amount-mismatch' });
});

test('classifyAlreadyPaidRecovery: 미결제 조회는 알람 없이 유예로 수렴한다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'not-paid', paymentStatus: 'CANCELLED' },
    attemptRecords: [{ billingAmount: 4900, creditAmount: 0 }],
  });

  assert.deepEqual(classification, { kind: 'defer' });
});

test('classifyAlreadyPaidRecovery: 조회 예외도 알람 없이 유예로 수렴한다', () => {
  const classification = classifyAlreadyPaidRecovery({
    lookup: { kind: 'error' },
    attemptRecords: [{ billingAmount: 4900, creditAmount: 0 }],
  });

  assert.deepEqual(classification, { kind: 'defer' });
});

test('splitBillingAmount: 0원 인보이스는 크레딧을 쓰지 않는다', () => {
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 0, creditBalance: 4900 }), { billingAmount: 0, creditAmount: 0 });
});

test('splitBillingAmount: 크레딧이 인보이스 금액 이상이면 전액을 크레딧으로 덮는다', () => {
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 4900, creditBalance: 4900 }), { billingAmount: 0, creditAmount: 4900 });
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 4900, creditBalance: 10_000 }), { billingAmount: 0, creditAmount: 4900 });
});

test('splitBillingAmount: 크레딧이 모자라면 나머지를 PG에 청구한다', () => {
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 4900, creditBalance: 2900 }), { billingAmount: 2000, creditAmount: 2900 });
});

test('splitBillingAmount: 크레딧 잔액이 없거나 음수면 전액을 PG에 청구한다', () => {
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 4900, creditBalance: 0 }), { billingAmount: 4900, creditAmount: 0 });
  assert.deepEqual(splitBillingAmount({ invoiceAmount: 4900, creditBalance: -100 }), { billingAmount: 4900, creditAmount: 0 });
});
