import { error } from '@sveltejs/kit';
import { and, eq, sql } from 'drizzle-orm';
import { countClaimable } from '$lib/server/claim.ts';
import { createDb, Judgments, Tasks } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform, locals }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const drafts = await db
    .select({ taskId: Judgments.taskId, kind: Tasks.kind, updatedAt: Judgments.updatedAt })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true)));

  const done = await db
    .select({ taskId: Judgments.taskId })
    .from(Judgments)
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, false)));

  const [taskTotal] = await db.select({ n: sql<number>`count(*)` }).from(Tasks);
  const remaining = await countClaimable(db, locals.email);

  return { email: locals.email, drafts, doneCount: done.length, total: taskTotal?.n ?? 0, remaining };
};
