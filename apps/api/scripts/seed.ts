#!/usr/bin/env node

import { PlanId } from '@/const';
import { db, Plans } from '@/db';
import { PlanAvailability } from '@/enums';

await db.transaction(async (tx) => {
  await tx.insert(Plans).values({
    id: PlanId.PLUS,
    name: 'Plus',
    fee: 4900,
    rules: {
      maxTotalCharacterCount: -1,
      maxTotalBlobSize: -1,
    },
  });

  await tx.insert(Plans).values({
    id: 'PL0PENXLE',
    name: 'PENXLE',
    fee: 0,
    rules: {},
    availability: PlanAvailability.PRIVATE,
  });
});

process.exit(0);
