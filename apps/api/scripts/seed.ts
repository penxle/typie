#!/usr/bin/env node

import { PlanId } from '@typie/lib/const';
import { PlanAvailability, PlanInterval } from '@typie/lib/enums';
import { db, Plans, TextReplacements } from '#/db/index.ts';
import { generateFractionalOrder } from '#/utils/index.ts';

const plans = [
  {
    id: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY,
    name: '타이피 FULL ACCESS (월간)',
    fee: 4900,
    availability: PlanAvailability.BILLING_KEY,
    interval: PlanInterval.MONTHLY,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
  {
    id: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY,
    name: '타이피 FULL ACCESS (연간)',
    fee: 49_000,
    availability: PlanAvailability.BILLING_KEY,
    interval: PlanInterval.YEARLY,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
  {
    id: PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE,
    name: '타이피 FULL ACCESS (월간)',
    fee: 6900,
    availability: PlanAvailability.IN_APP_PURCHASE,
    interval: PlanInterval.MONTHLY,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
  {
    id: PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE,
    name: '타이피 FULL ACCESS (연간)',
    fee: 69_000,
    availability: PlanAvailability.IN_APP_PURCHASE,
    interval: PlanInterval.YEARLY,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
  {
    id: PlanId.FULL_ACCESS_TRIAL,
    name: '타이피 FULL ACCESS (체험)',
    fee: 0,
    availability: PlanAvailability.TRIAL,
    interval: PlanInterval.TRIAL,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
  {
    id: PlanId.LIFETIME_ACCESS,
    name: '타이피 LIFETIME ACCESS',
    fee: 0,
    availability: PlanAvailability.MANUAL,
    interval: PlanInterval.LIFETIME,
    rule: { maxTotalCharacterCount: -1, maxTotalBlobSize: -1 },
  },
];

for (const plan of plans) {
  await db
    .insert(Plans)
    .values(plan)
    .onConflictDoUpdate({
      target: Plans.id,
      set: {
        name: plan.name,
        fee: plan.fee,
        availability: plan.availability,
        interval: plan.interval,
        rule: plan.rule,
      },
    });
}

// spell-checker:disable
const textReplacementPresets = [
  { id: 'TXR0DASH', match: '--', substitute: '\u2014', note: '하이픈 두 개를 줄표(\u2014)로' },
  { id: 'TXR0ELLIPSIS', match: '...', substitute: '\u2026', note: '마침표 세 개를 말줄임표(\u2026)로' },
  { id: 'TXR0SQUOTEOPEN', match: "(?<!\u2018[^\u2019]*)'", substitute: '\u2018', regex: true, note: '스마트 따옴표 (여는 홑따옴표)' },
  { id: 'TXR0SQUOTECLOSE', match: "(?<=\u2018[^\u2019]*)'", substitute: '\u2019', regex: true, note: '스마트 따옴표 (닫는 홑따옴표)' },
  { id: 'TXR0DQUOTEOPEN', match: '(?<!\u201C[^\u201D]*)"', substitute: '\u201C', regex: true, note: '스마트 따옴표 (여는 쌍따옴표)' },
  { id: 'TXR0DQUOTECLOSE', match: '(?<=\u201C[^\u201D]*)"', substitute: '\u201D', regex: true, note: '스마트 따옴표 (닫는 쌍따옴표)' },
];
// spell-checker:enable

let lastOrder: string | undefined;

for (const preset of textReplacementPresets) {
  const order = generateFractionalOrder({ lower: lastOrder, upper: undefined });
  lastOrder = order;

  await db
    .insert(TextReplacements)
    .values({ ...preset, preset: true, order })
    .onConflictDoUpdate({
      target: TextReplacements.id,
      set: {
        match: preset.match,
        substitute: preset.substitute,
        regex: preset.regex ?? false,
        note: preset.note,
        order,
      },
    });
}

process.exit(0);
