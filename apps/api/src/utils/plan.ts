import { PlanId } from '@typie/lib/const';
import { SubscriptionState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, eq, gt, inArray, or } from 'drizzle-orm';
import { db, first, firstOrThrow, Subscriptions, UserTrials } from '#/db/index.ts';
import type { Transaction } from '#/db/index.ts';

export const ACTIVE_SUBSCRIPTION_STATES = [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD];

type AssertActiveSubscriptionParams = {
  userId: string;
};

export const hasActiveSubscription = async ({ userId }: { userId: string }) => {
  // WILL_EXPIRE 는 만료일이 지나면(해지 확정·일시중지·보류 등) 권한이 없어야 한다. ACTIVE/IN_GRACE_PERIOD 는 상태만으로 판정한다.
  const subscription = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .where(
      and(
        eq(Subscriptions.userId, userId),
        or(
          inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.IN_GRACE_PERIOD]),
          and(eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE), gt(Subscriptions.expiresAt, dayjs())),
        ),
      ),
    )
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
