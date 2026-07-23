import { desc, eq, sql } from 'drizzle-orm';
import { Judgments, Rounds, Tasks } from './db/index.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

// 라운드 진행은 태스크별 min(확정 판정 수, 필요 수)의 합으로 센다 — 초과 배정으로 생긴
// 잉여 판정을 원시 count로 세면 진행률이 부풀고 필요 총합을 초과할 수도 있다.
// 범위는 최신 라운드 하나다 — 전 라운드 합산은 새 라운드 시작 직후 진행률을 부풀린다.
export const effectiveProgress = async (db: Db): Promise<{ done: number; required: number; roundId: string | null }> => {
  const [latest] = await db.select({ id: Rounds.id }).from(Rounds).orderBy(desc(Rounds.createdAt)).limit(1);
  if (!latest) {
    return { done: 0, required: 0, roundId: null };
  }

  const tasks = await db
    .select({ id: Tasks.id, requiredJudgments: Tasks.requiredJudgments })
    .from(Tasks)
    .where(eq(Tasks.roundId, latest.id));
  const counts = await db
    .select({ taskId: Judgments.taskId, n: sql<number>`count(*)` })
    .from(Judgments)
    .where(eq(Judgments.draft, false))
    .groupBy(Judgments.taskId);
  const byTask = new Map(counts.map((c) => [c.taskId, c.n]));

  let done = 0;
  let required = 0;
  for (const task of tasks) {
    const req = task.requiredJudgments ?? 1;
    required += req;
    done += Math.min(byTask.get(task.id) ?? 0, req);
  }
  return { done, required, roundId: latest.id };
};
