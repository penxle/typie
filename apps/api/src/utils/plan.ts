import dayjs from 'dayjs';
import { and, eq, gt, or } from 'drizzle-orm';
import { db, first, Plans, UserPlans } from '@/db';
import { defaultPlanRules } from '@/db/schemas/json';
import { UserPlanState } from '@/enums';
import type { PlanRules } from '@/db/schemas/json';

type GetPlanParams<T extends keyof PlanRules> = {
  userId: string;
  rule: T;
};

const getPlanRuleValue = async <T extends keyof PlanRules>({ userId, rule }: GetPlanParams<T>) => {
  const plan = await db
    .select({ rules: Plans.rules })
    .from(Plans)
    .innerJoin(UserPlans, eq(Plans.id, UserPlans.planId))
    .where(
      and(
        eq(UserPlans.userId, userId),
        or(eq(UserPlans.state, UserPlanState.ACTIVE), and(eq(UserPlans.state, UserPlanState.CANCELED), gt(UserPlans.expiresAt, dayjs()))),
      ),
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
