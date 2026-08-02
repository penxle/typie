import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import test from 'node:test';
import { PlanInterval } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { computeNextPeriodEnd, floorToHourKst, LIFETIME_PERIOD_END } from './billing-period.ts';

test('앵커 복원: 1/31 → 2/28 → 3/31 → 4/30 → 5/31, 시각은 앵커 14:00 고정', () => {
  const anchor = dayjs.kst('2026-01-31 14:00');

  const feb = computeNextPeriodEnd({ periodStartsAt: anchor, interval: PlanInterval.MONTHLY, billingAnchorAt: anchor });
  assert.equal(feb.kst().format('MM-DD HH:mm'), '02-28 14:00');

  const mar = computeNextPeriodEnd({ periodStartsAt: feb, interval: PlanInterval.MONTHLY, billingAnchorAt: anchor });
  assert.equal(mar.kst().format('MM-DD HH:mm'), '03-31 14:00');

  const apr = computeNextPeriodEnd({ periodStartsAt: mar, interval: PlanInterval.MONTHLY, billingAnchorAt: anchor });
  assert.equal(apr.kst().format('MM-DD HH:mm'), '04-30 14:00');

  const may = computeNextPeriodEnd({ periodStartsAt: apr, interval: PlanInterval.MONTHLY, billingAnchorAt: anchor });
  assert.equal(may.kst().format('MM-DD HH:mm'), '05-31 14:00');
});

test('clamp 제거: 7/15 14:32 가입(앵커 14:00) → 8/15 14:00, 자정 아님', () => {
  const startsAt = dayjs.kst('2026-07-15 14:32');
  const anchor = floorToHourKst(startsAt);

  const end = computeNextPeriodEnd({ periodStartsAt: startsAt, interval: PlanInterval.MONTHLY, billingAnchorAt: anchor });

  assert.equal(end.kst().format('MM-DD HH:mm:ss'), '08-15 14:00:00');
});

test('YEARLY: 2/29 앵커(윤년) → 이듬해 2/28', () => {
  const anchor = dayjs.kst('2024-02-29 09:00');

  const next = computeNextPeriodEnd({ periodStartsAt: anchor, interval: PlanInterval.YEARLY, billingAnchorAt: anchor });

  assert.equal(next.kst().format('YYYY-MM-DD HH:mm'), '2025-02-28 09:00');
});

test('floorToHourKst: 분·초는 내리고, 정각 입력은 불변', () => {
  const withMinutesAndSeconds = dayjs.kst('2026-01-31 14:32:07');
  assert.equal(floorToHourKst(withMinutesAndSeconds).format('HH:mm:ss'), '14:00:00');

  const onTheHour = dayjs.kst('2026-01-31 14:00:00');
  assert.ok(floorToHourKst(onTheHour).isSame(onTheHour));
});

test('TRIAL·LIFETIME interval은 throw', () => {
  const anchor = dayjs.kst('2026-01-31 14:00');

  assert.throws(() => computeNextPeriodEnd({ periodStartsAt: anchor, interval: PlanInterval.TRIAL, billingAnchorAt: anchor }));
  assert.throws(() => computeNextPeriodEnd({ periodStartsAt: anchor, interval: PlanInterval.LIFETIME, billingAnchorAt: anchor }));
});

test('LIFETIME_PERIOD_END sentinel은 9999-12-31', () => {
  assert.equal(LIFETIME_PERIOD_END.toISOString(), dayjs('9999-12-31T00:00:00.000Z').toISOString());
});
