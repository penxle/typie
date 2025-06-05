import { and, eq, inArray } from 'drizzle-orm';
import { defaultPlanRules } from '@/const';
import { db, first, Plans, Subscriptions } from '@/db';
import { SubscriptionState } from '@/enums';
import type { PlanRules } from '@/db/schemas/json';

type GetPlanParams<T extends keyof PlanRules> = {
  userId: string;
  rule: T;
};

const getPlanRuleValue = async <T extends keyof PlanRules>({ userId, rule }: GetPlanParams<T>) => {
  const plan = await db
    .select({ rules: Plans.rule })
    .from(Plans)
    .innerJoin(Subscriptions, eq(Plans.id, Subscriptions.planId))
    .where(
      and(eq(Subscriptions.userId, userId), inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.IN_GRACE_PERIOD])),
    )
    .then(first);

  return plan?.rules[rule] === undefined ? defaultPlanRules[rule] : plan.rules[rule];
};

export const assertPlanRule = async <T extends keyof PlanRules>({ userId, rule }: GetPlanParams<T>) => {
  const value = await getPlanRuleValue({ userId, rule });

  if (value === -1) {
    return;
  }
};
