import { defaultPlanRules } from '@typie/lib/const';
import { SubscriptionState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, eq, inArray } from 'drizzle-orm';
import { db, first, Plans, Subscriptions } from '#/db/index.ts';
import { getUserUsage } from './user.ts';
import type { PlanRules } from '#/db/schemas/json.ts';

export const ACTIVE_SUBSCRIPTION_STATES = [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD];

type AssertActiveSubscriptionParams = {
  userId: string;
};
export const assertActiveSubscription = async ({ userId }: AssertActiveSubscriptionParams) => {
  const subscription = await db
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .where(and(eq(Subscriptions.userId, userId), inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES)))
    .then(first);

  if (!subscription) {
    throw new TypieError({ code: 'subscription_required', status: 403 });
  }
};

type GetPlanParams<T extends keyof PlanRules> = {
  userId: string;
  rule: T;
};

export const getPlanRuleValue = async <T extends keyof PlanRules>({ userId, rule }: GetPlanParams<T>) => {
  const plan = await db
    .select({ rules: Plans.rule })
    .from(Plans)
    .innerJoin(Subscriptions, eq(Plans.id, Subscriptions.planId))
    .where(and(eq(Subscriptions.userId, userId), inArray(Subscriptions.state, ACTIVE_SUBSCRIPTION_STATES)))
    .then(first);

  return plan?.rules[rule] === undefined ? defaultPlanRules[rule] : plan.rules[rule];
};

type AssertPlanRuleParams<T extends keyof PlanRules> = {
  userId: string;
  rule: T;
};
export const assertPlanRule = async <T extends keyof PlanRules>({ userId, rule }: AssertPlanRuleParams<T>) => {
  const value = await getPlanRuleValue({ userId, rule });

  if (value === -1) {
    return;
  }

  switch (rule) {
    case 'maxTotalCharacterCount': {
      const usage = await getUserUsage({ userId });
      if (usage.totalCharacterCount >= value) {
        throw new TypieError({ code: 'character_count_limit_exceeded' });
      }

      break;
    }

    case 'maxTotalBlobSize': {
      const usage = await getUserUsage({ userId });
      if (usage.totalBlobSize >= value) {
        throw new TypieError({ code: 'blob_size_limit_exceeded' });
      }

      break;
    }
  }
};
