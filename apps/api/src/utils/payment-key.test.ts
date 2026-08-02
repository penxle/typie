import assert from 'node:assert/strict';
import test from 'node:test';
import dayjs from 'dayjs';
import { derivePaymentKey } from './payment-key.ts';

test('derivePaymentKey: lineageId와 servicePeriodStartsAt epoch ms를 밑줄로 결합한다', () => {
  const key = derivePaymentKey('sub_x', dayjs('2026-08-01T01:00:00.000Z'));

  assert.equal(key, 'sub_x_1785546000000');
});
