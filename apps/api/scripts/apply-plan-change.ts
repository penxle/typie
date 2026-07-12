#!/usr/bin/env node

import { PlanId } from '@typie/lib/const';
import { SubscriptionState, UserState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, inArray, lt, notExists } from 'drizzle-orm';
import { db, Plans, Subscriptions, Users, UserTrials } from '#/db/index.ts';
import { ACTIVE_SUBSCRIPTION_STATES, createTrialSubscription } from '#/utils/plan.ts';

const TRIAL_STARTS_AT = dayjs('2026-07-13T00:00:00+09:00');
const TRIAL_EXPIRES_AT = dayjs('2026-07-27T00:00:00+09:00');

const NEW_FEES = [
  { id: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY, fee: 2900 },
  { id: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY, fee: 29_000 },
  { id: PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE, fee: 2900 },
  { id: PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE, fee: 29_000 },
];

for (const { id, fee } of NEW_FEES) {
  await db.update(Plans).set({ fee }).where(eq(Plans.id, id));
  console.log(`✓ ${id} → ${fee}원`);
}

const extended = await db
  .update(Subscriptions)
  .set({ expiresAt: TRIAL_EXPIRES_AT })
  .where(
    and(
      eq(Subscriptions.planId, PlanId.FULL_ACCESS_TRIAL),
      eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE),
      lt(Subscriptions.expiresAt, TRIAL_EXPIRES_AT),
    ),
  )
  .returning({ userId: Subscriptions.userId });

if (extended.length > 0) {
  await db
    .update(UserTrials)
    .set({ expiresAt: TRIAL_EXPIRES_AT })
    .where(
      inArray(
        UserTrials.userId,
        extended.map((e) => e.userId),
      ),
    );
}
console.log(`✓ 진행 중 트라이얼 연장: ${extended.length}명`);

const targets = await db
  .select({ id: Users.id })
  .from(Users)
  .where(
    and(
      eq(Users.state, UserState.ACTIVE),
      notExists(
        db
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(and(eq(Subscriptions.userId, Users.id), inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES))),
      ),
    ),
  );

console.log(`백필 대상: ${targets.length}명`);

let processed = 0;
for (const target of targets) {
  await db.transaction(async (tx) => {
    await createTrialSubscription(tx, { userId: target.id, startsAt: TRIAL_STARTS_AT, expiresAt: TRIAL_EXPIRES_AT });
  });
  processed++;
  if (processed % 500 === 0) {
    console.log(`  ${processed}/${targets.length}`);
  }
}

console.log(`✓ 트라이얼 백필: ${processed}명`);
process.exit(0);
