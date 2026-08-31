import assert from 'node:assert/strict';
import { test } from 'node:test';
import { evaluatePrismAccess } from './prism-access-core.ts';

test('구독·동의가 모두 참이고 크레딧 검사가 없으면 ok', () => {
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: null }), 'ok');
});

test('판정 순서는 구독 → 동의 → 크레딧', () => {
  assert.equal(evaluatePrismAccess({ entitled: false, aiOptIn: false, credit: null }), 'subscription_required');
  assert.equal(evaluatePrismAccess({ entitled: false, aiOptIn: true, credit: { balance: 0, required: 1 } }), 'subscription_required');
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: false, credit: null }), 'ai_opt_in_required');
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: false, credit: { balance: -1, required: 0 } }), 'ai_opt_in_required');
});

test('크레딧: 잔액이 필요량 이상이면 ok', () => {
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: 0, required: 0 } }), 'ok');
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: -1, required: 0 } }), 'prism_credit_insufficient');
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: 1, required: 1 } }), 'ok');
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: 0, required: 1 } }), 'prism_credit_insufficient');
  assert.equal(
    evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: 639_000, required: 640_000 } }),
    'prism_credit_insufficient',
  );
  assert.equal(evaluatePrismAccess({ entitled: true, aiOptIn: true, credit: { balance: 640_000, required: 640_000 } }), 'ok');
});
