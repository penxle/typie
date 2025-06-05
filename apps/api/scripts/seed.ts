#!/usr/bin/env node

import { PlanId } from '@/const';
import { db, Plans } from '@/db';
import { PlanAvailability, PlanInterval } from '@/enums';

await db.transaction(async (tx) => {
  await tx.insert(Plans).values({
    id: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY,
    name: '타이피 FULL ACCESS (월간)',
    fee: 4900,
    availability: PlanAvailability.BILLING_KEY,
    interval: PlanInterval.MONTHLY,
    rule: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });

  await tx.insert(Plans).values({
    id: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY,
    name: '타이피 FULL ACCESS (연간)',
    fee: 49_000,
    availability: PlanAvailability.BILLING_KEY,
    interval: PlanInterval.YEARLY,
    rule: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });

  await tx.insert(Plans).values({
    id: PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE,
    name: '타이피 FULL ACCESS (월간)',
    fee: 6900,
    availability: PlanAvailability.IN_APP_PURCHASE,
    interval: PlanInterval.MONTHLY,
    rule: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });

  await tx.insert(Plans).values({
    id: PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE,
    name: '타이피 FULL ACCESS (연간)',
    fee: 69_000,
    availability: PlanAvailability.IN_APP_PURCHASE,
    interval: PlanInterval.YEARLY,
    rule: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });
});

process.exit(0);
