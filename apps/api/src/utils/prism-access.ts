import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { db, first, UserPreferences } from '#/db/index.ts';
import { env } from '#/env.ts';
import { hasActiveSubscription } from './plan.ts';
import { evaluatePrismAccess, parseAllowlist } from './prism-access-core.ts';

export const assertPrismAccess = async ({ userId }: { userId: string }) => {
  const preference = await db
    .select({ value: UserPreferences.value })
    .from(UserPreferences)
    .where(eq(UserPreferences.userId, userId))
    .then(first);

  const code = evaluatePrismAccess({
    allowlisted: parseAllowlist(env.PRISM_BETA_USER_IDS).includes(userId),
    entitled: await hasActiveSubscription({ userId }),
    aiOptIn: preference?.value.aiOptIn === true,
  });

  if (code !== 'ok') {
    throw new TypieError({ code, status: 403 });
  }
};
