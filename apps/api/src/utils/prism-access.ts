import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { db, first, UserPreferences } from '#/db/index.ts';
import { hasActiveSubscription } from './plan.ts';
import { evaluatePrismAccess } from './prism-access-core.ts';
import { readPrismCreditBalance } from './prism-credit.ts';

export const assertPrismAccess = async ({ userId, credit }: { userId: string; credit?: { required: number } }) => {
  const preference = await db
    .select({ value: UserPreferences.value })
    .from(UserPreferences)
    .where(eq(UserPreferences.userId, userId))
    .then(first);

  const code = evaluatePrismAccess({
    entitled: await hasActiveSubscription({ userId }),
    aiOptIn: preference?.value.aiOptIn === true,
    credit: credit ? { balance: await readPrismCreditBalance(db, userId).then(({ total }) => total), required: credit.required } : null,
  });

  if (code !== 'ok') {
    throw new TypieError({ code, status: 403 });
  }
};
