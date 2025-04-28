import { and, count, eq } from 'drizzle-orm';
import { db, Entities, first, Plans, Posts, UserPlans } from '@/db';
import { defaultPlanRules } from '@/db/schemas/json';
import { EntityState } from '@/enums';
import { TypieError } from '@/errors';
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
    .where(eq(UserPlans.userId, userId))
    .then(first);

  return plan?.rules[rule] === undefined ? defaultPlanRules[rule] : plan.rules[rule];
};

export const assertPlanRule = async <T extends keyof PlanRules>({ userId, rule }: GetPlanParams<T>) => {
  const value = await getPlanRuleValue({ userId, rule });

  if (value === -1) {
    return;
  }

  switch (rule) {
    case 'maxPostCount': {
      const postCount = await db
        .select({ count: count() })
        .from(Posts)
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
        .then((result) => result[0]?.count ?? 0);

      if (postCount >= value) {
        throw new TypieError({ code: 'max_post_count_reached' });
      }

      break;
    }
  }
};
