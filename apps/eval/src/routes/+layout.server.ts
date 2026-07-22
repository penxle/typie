import { error, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { createDb, EvaluatorConsents } from '$lib/server/db/index.ts';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ platform, locals, url }) => {
  if (url.pathname.startsWith('/consent') || url.pathname.startsWith('/admin')) {
    return {};
  }
  if (!platform) {
    error(500, 'platform unavailable');
  }
  const db = createDb(platform.env.DB);
  const [consent] = await db
    .select({ email: EvaluatorConsents.email })
    .from(EvaluatorConsents)
    .where(eq(EvaluatorConsents.email, locals.email));
  if (!consent) {
    redirect(302, '/consent');
  }
  return {};
};
