import { and, asc, eq, notInArray, sql } from 'drizzle-orm';
import { Judgments, Tasks } from './db/index.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

const claimableQuery = (db: Db, email: string) => {
  const mine = db.select({ taskId: Judgments.taskId }).from(Judgments).where(eq(Judgments.evaluatorEmail, email));

  const counts = db
    .select({ taskId: Judgments.taskId, total: sql<number>`count(*)`.as('total') })
    .from(Judgments)
    .groupBy(Judgments.taskId)
    .as('counts');

  return db
    .select({ id: Tasks.id })
    .from(Tasks)
    .leftJoin(counts, eq(counts.taskId, Tasks.id))
    .where(
      and(
        notInArray(Tasks.id, mine),
        sql`(${Tasks.requiredJudgments} is null or coalesce(${counts.total}, 0) < ${Tasks.requiredJudgments})`,
      ),
    )
    .orderBy(asc(Tasks.createdAt));
};

export const claimNextTask = async (db: Db, email: string): Promise<string | null> => {
  const [task] = await claimableQuery(db, email).limit(1);
  return task?.id ?? null;
};

export const countClaimable = async (db: Db, email: string): Promise<number> => {
  const rows = await claimableQuery(db, email);
  return rows.length;
};
