import { error, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, EvaluatorConsents } from '$lib/server/db/index.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform, locals }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }
  const db = createDb(platform.env.DB);
  const [consent] = await db
    .select({ email: EvaluatorConsents.email })
    .from(EvaluatorConsents)
    .where(eq(EvaluatorConsents.email, locals.email));
  if (consent) {
    redirect(302, '/');
  }
  return { email: locals.email };
};

export const actions: Actions = {
  default: async ({ platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }
    const db = createDb(platform.env.DB);
    await db.insert(EvaluatorConsents).values({ email: locals.email }).onConflictDoNothing();
    redirect(302, '/');
  },
};
