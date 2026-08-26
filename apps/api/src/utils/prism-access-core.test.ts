import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluatePrismAccess, parseAllowlist } from './prism-access-core.ts';

test('allowlist·구독·동의가 모두 참이고 크레딧 검사가 없으면 ok', () => {
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: null }), 'ok');
});

test('parseAllowlist: 쉼표 구분·공백 정리·빈 항목 제거', () => {
  assert.deepEqual(parseAllowlist('U1, U2 ,,U3'), ['U1', 'U2', 'U3']);
  assert.deepEqual(parseAllowlist(''), []);
  assert.deepEqual(parseAllowlist(' , '), []);
});

test('판정 순서는 allowlist → 구독 → 동의 → 크레딧', () => {
  assert.equal(evaluatePrismAccess({ allowlisted: false, entitled: false, aiOptIn: false, credit: null }), 'prism_beta_required');
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: false, aiOptIn: false, credit: null }), 'subscription_required');
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: false, credit: null }), 'ai_opt_in_required');
  assert.equal(
    evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: false, credit: { balance: -1, required: 0 } }),
    'ai_opt_in_required',
  );
});

test('크레딧: 잔액이 문턱 이상이면 ok, 미만이면 prism_credit_insufficient', () => {
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: 0, required: 0 } }), 'ok');
  assert.equal(
    evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: -1, required: 0 } }),
    'prism_credit_insufficient',
  );
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: 1, required: 1 } }), 'ok');
  assert.equal(
    evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: 0, required: 1 } }),
    'prism_credit_insufficient',
  );
  assert.equal(
    evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: 639_000, required: 640_000 } }),
    'prism_credit_insufficient',
  );
  assert.equal(
    evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true, credit: { balance: 640_000, required: 640_000 } }),
    'ok',
  );
});
