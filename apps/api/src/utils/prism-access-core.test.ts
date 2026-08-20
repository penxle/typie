import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluatePrismAccess, parseAllowlist } from './prism-access-core.ts';

test('allowlist·구독·동의가 모두 참이면 ok', () => {
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: true }), 'ok');
});

test('parseAllowlist: 쉼표 구분·공백 정리·빈 항목 제거', () => {
  assert.deepEqual(parseAllowlist('U1, U2 ,,U3'), ['U1', 'U2', 'U3']);
  assert.deepEqual(parseAllowlist(''), []);
  assert.deepEqual(parseAllowlist(' , '), []);
});

test('판정 순서는 allowlist → 구독 → 동의', () => {
  assert.equal(evaluatePrismAccess({ allowlisted: false, entitled: false, aiOptIn: false }), 'prism_beta_required');
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: false, aiOptIn: false }), 'subscription_required');
  assert.equal(evaluatePrismAccess({ allowlisted: true, entitled: true, aiOptIn: false }), 'ai_opt_in_required');
});
