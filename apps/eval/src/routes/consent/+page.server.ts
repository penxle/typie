import { error, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, Evaluators } from '../../../core/db.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform, locals }) => {
  if (!platform) error(500, 'platform unavailable');
  const [existing] = await createDb(platform.env.DB).select().from(Evaluators).where(eq(Evaluators.email, locals.email));
  if (existing) redirect(302, '/');
  return { email: locals.email };
};

export const actions: Actions = {
  // 동의만으로는 평가자가 되지 않는다 — 어드민이 명단에서 켜야 배정이 열린다(사후승인).
  default: async ({ platform, locals }) => {
    if (!platform) error(500, 'platform unavailable');
    await createDb(platform.env.DB).insert(Evaluators).values({ email: locals.email }).onConflictDoNothing();
    redirect(303, '/');
  },
};
