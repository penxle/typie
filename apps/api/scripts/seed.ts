#!/usr/bin/env node

import { PlanId } from '@/const';
import { db, Plans } from '@/db';
import { PlanAvailability, PlanInterval } from '@/enums';

await db.transaction(async (tx) => {
  await tx.insert(Plans).values({
    id: PlanId.FULL_ACCESS_1MONTH,
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
    id: PlanId.FULL_ACCESS_1YEAR,
    name: '타이피 FULL ACCESS (연간)',
    fee: 49_000,
    availability: PlanAvailability.BILLING_KEY,
    interval: PlanInterval.YEARLY,
    rule: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });
});

process.exit(0);
