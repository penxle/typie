import { error } from '@sveltejs/kit';
import { desc, eq, inArray, sql } from 'drizzle-orm';
import { createDb, Documents, Judgments, Rounds, Tasks } from '$lib/server/db/index.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const rounds = await db.select().from(Rounds).orderBy(desc(Rounds.createdAt));
  const tasks =
    rounds.length > 0
      ? await db
          .select()
          .from(Tasks)
          .where(
            inArray(
              Tasks.roundId,
              rounds.map((r) => r.id),
            ),
          )
      : [];

  const documentIds = [...new Set(tasks.map((t) => t.documentId))];
  const documents =
    documentIds.length > 0
      ? await db
          .select({ id: Documents.id, characterCount: Documents.characterCount, corpusVersion: Documents.corpusVersion })
          .from(Documents)
          .where(inArray(Documents.id, documentIds))
      : [];
  const docById = new Map(documents.map((d) => [d.id, d]));

  const confirmedCounts = await db
    .select({ taskId: Judgments.taskId, n: sql<number>`count(*)` })
    .from(Judgments)
    .where(eq(Judgments.draft, false))
    .groupBy(Judgments.taskId);
  const confirmedByTask = new Map(confirmedCounts.map((c) => [c.taskId, c.n]));

  return {
    rounds: rounds.map((round) => ({
      id: round.id,
      stage: round.stage,
      createdAt: round.createdAt.toISOString(),
      tasks: tasks
        .filter((t) => t.roundId === round.id)
        .map((t) => ({
          id: t.id,
          documentId: t.documentId,
          kind: t.kind,
          characterCount: docById.get(t.documentId)?.characterCount ?? 0,
          corpusVersion: docById.get(t.documentId)?.corpusVersion ?? '?',
          required: t.requiredJudgments ?? 1,
          confirmed: confirmedByTask.get(t.id) ?? 0,
        })),
    })),
  };
};
