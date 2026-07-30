import assert from 'node:assert/strict';
import test from 'node:test';
import { BillingKeyType, PlanInterval } from './enums.ts';
import { supportsPlanInterval } from './plan.ts';

test('카드는 모든 주기를 지원한다', () => {
  assert.equal(supportsPlanInterval(BillingKeyType.CARD, PlanInterval.MONTHLY), true);
  assert.equal(supportsPlanInterval(BillingKeyType.CARD, PlanInterval.YEARLY), true);
});

test('카카오페이는 월간만 지원한다', () => {
  assert.equal(supportsPlanInterval(BillingKeyType.KAKAOPAY, PlanInterval.MONTHLY), true);
});

test('카카오페이는 연간을 지원하지 않는다', () => {
  assert.equal(supportsPlanInterval(BillingKeyType.KAKAOPAY, PlanInterval.YEARLY), false);
});

test('카카오페이는 화이트리스트 밖 주기를 지원하지 않는다', () => {
  assert.equal(supportsPlanInterval(BillingKeyType.KAKAOPAY, PlanInterval.LIFETIME), false);
  assert.equal(supportsPlanInterval(BillingKeyType.KAKAOPAY, PlanInterval.TRIAL), false);
});
