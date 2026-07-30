import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyEasyPayBillingKey } from './billing-key.ts';
import type { BillingKeyIssuance } from './billing-key.ts';

const params = { userId: 'U0000', channelKey: 'channel-key-kakaopay' };

const issuance = (overrides: Partial<BillingKeyIssuance> = {}): BillingKeyIssuance => ({
  status: 'ISSUED',
  customerId: 'U0000',
  channelKeys: ['channel-key-kakaopay'],
  ...overrides,
});

test('발급 완료 + 본인 + 카카오페이 채널이면 수용한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance(), params), { ok: true });
});

test('발급 완료 상태가 아니면 거부한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance({ status: 'DELETED' }), params), { ok: false, reason: 'not_issued' });
});

test('다른 유저의 키면 거부한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance({ customerId: 'U9999' }), params), { ok: false, reason: 'customer_mismatch' });
});

test('고객 아이디가 없으면 거부한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance({ customerId: undefined }), params), { ok: false, reason: 'customer_missing' });
});

test('다른 채널에서 발급된 키면 거부한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance({ channelKeys: ['channel-key-other'] }), params), {
    ok: false,
    reason: 'channel_mismatch',
  });
});

test('채널 키가 비어 있으면 거부한다', () => {
  assert.deepEqual(verifyEasyPayBillingKey(issuance({ channelKeys: [] }), params), { ok: false, reason: 'channel_mismatch' });
});
