import { PlanId } from '@typie/lib/const';
import { SubscriptionState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, eq, inArray } from 'drizzle-orm';
import { db, first, firstOrThrow, Subscriptions, UserTrials } from '#/db/index.ts';
import type dayjs from 'dayjs';
import type { Transaction } from '#/db/index.ts';

export const ACTIVE_SUBSCRIPTION_STATES = [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD];

type AssertActiveSubscriptionParams = {
  userId: string;
};

export const hasActiveSubscription = async ({ userId }: { userId: string }) => {
  const subscription = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .where(and(eq(Subscriptions.userId, userId), inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES)))
    .then(first);

  return !!subscription;
};

export const assertActiveSubscription = async ({ userId }: AssertActiveSubscriptionParams) => {
  if (!(await hasActiveSubscription({ userId }))) {
    throw new TypieError({ code: 'subscription_required', status: 403 });
  }
};

type CreateTrialSubscriptionParams = {
  userId: string;
  startsAt: dayjs.Dayjs;
  expiresAt: dayjs.Dayjs;
};

export const createTrialSubscription = async (tx: Transaction, { userId, startsAt, expiresAt }: CreateTrialSubscriptionParams) => {
  const subscription = await tx
    .insert(Subscriptions)
    .values({
      userId,
      planId: PlanId.FULL_ACCESS_TRIAL,
      startsAt,
      expiresAt,
      renewedAt: startsAt,
      state: SubscriptionState.WILL_EXPIRE,
    })
    .returning()
    .then(firstOrThrow);

  await tx
    .insert(UserTrials)
    .values({ userId, subscriptionId: subscription.id, startedAt: startsAt, expiresAt })
    .onConflictDoUpdate({
      target: UserTrials.userId,
      set: { subscriptionId: subscription.id, startedAt: startsAt, expiresAt },
    });

  return subscription;
};
