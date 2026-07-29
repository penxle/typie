import { and, eq, inArray, lt } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { Evaluators, inChunks, JudgmentItems, Judgments, Rounds, TaskReleases, Tasks } from '../../../core/db.ts';
import type { Db } from '../../../core/db.ts';

// 중복 배정이 없어지면서 규칙이 셋으로 줄었다 — 명단에 있어야 받고, 아무도 안 가져간 것만
// 받고, 내가 반납한 것은 다시 받지 않는다. 소프트 캡·최소 몫 예약·따라잡기는 전부 "같은 것을
// 여러 명이 본다"를 전제한 장치였다.
// 후보 중에서는 무작위로 고른다 — 생성 순서대로 주면 표집 순서(장르 뭉침)가 배정 순서에
// 그대로 실리고, 평가자마다 같은 앞머리부터 읽게 된다.
export const pickTask = (
  tasks: { id: string }[],
  taken: Set<string>,
  released: Set<string>,
  rng: () => number = Math.random,
): string | null => {
  const candidates = tasks.filter((t) => !taken.has(t.id) && !released.has(t.id));
  return candidates.length > 0 ? candidates[Math.floor(rng() * candidates.length)].id : null;
};

export const isEvaluator = async (db: Db, email: string): Promise<boolean> => {
  const [row] = await db.select({ evaluating: Evaluators.evaluating }).from(Evaluators).where(eq(Evaluators.email, email)).limit(1);
  return row?.evaluating === true;
};

// 손대지 않은 예약은 태스크를 영구히 잠근다 — 배정 시점에 기회주의적으로 회수한다.
const RESERVATION_TTL_HOURS = 2;

const reclaimExpiredReservations = async (db: Db): Promise<void> => {
  const cutoff = new Date(Date.now() - RESERVATION_TTL_HOURS * 3600 * 1000);
  const stale = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .where(and(eq(Judgments.draft, true), eq(Judgments.elapsedSeconds, 0), lt(Judgments.updatedAt, cutoff)));
  if (stale.length === 0) return;

  const ids = stale.map((s) => s.id);
  // 항목이 하나라도 달린 예약은 사람이 손댄 것이므로 남긴다.
  const touched = await inChunks(ids, (chunk) =>
    db.select({ judgmentId: JudgmentItems.judgmentId }).from(JudgmentItems).where(inArray(JudgmentItems.judgmentId, chunk)),
  );
  const touchedIds = new Set(touched.map((t) => t.judgmentId));
  const empty = ids.filter((id) => !touchedIds.has(id));
  await inChunks(empty, (chunk) => db.delete(Judgments).where(inArray(Judgments.id, chunk)).returning({ id: Judgments.id }));
};

export const hasOpenDraft = async (db: Db, email: string): Promise<boolean> => {
  const [row] = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .where(and(eq(Judgments.evaluatorEmail, email), eq(Judgments.draft, true)))
    .limit(1);
  return row !== undefined;
};

export const claimTask = async (db: Db, roundId: string, email: string): Promise<string | null> => {
  if (!(await isEvaluator(db, email))) return null;

  await reclaimExpiredReservations(db);
  // 이어서 할 것이 있으면 새로 받지 않는다 — 받아만 두고 미룬 태스크가 남을 잠근다.
  if (await hasOpenDraft(db, email)) return null;

  const [round] = await db.select({ active: Rounds.active }).from(Rounds).where(eq(Rounds.id, roundId)).limit(1);
  if (!round?.active) return null;

  const tasks = await db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, roundId));
  if (tasks.length === 0) return null;
  const ids = tasks.map((t) => t.id);

  const taken = await inChunks(ids, (chunk) =>
    db.select({ taskId: Judgments.taskId }).from(Judgments).where(inArray(Judgments.taskId, chunk)),
  );
  const released = await inChunks(ids, (chunk) =>
    db
      .select({ taskId: TaskReleases.taskId })
      .from(TaskReleases)
      .where(and(eq(TaskReleases.evaluatorEmail, email), inArray(TaskReleases.taskId, chunk))),
  );

  const takenIds = new Set(taken.map((t) => t.taskId));
  const releasedIds = new Set(released.map((r) => r.taskId));

  // taskId unique 제약이 경합을 막는다 — 두 평가자가 동시에 같은 것을 집으면 뒤엣것이 튕기고
  // 그 자리에서 다음 후보로 넘어간다. 후보가 소진되면 pickTask가 null을 돌려 끝난다.
  for (;;) {
    const taskId = pickTask(tasks, takenIds, releasedIds);
    if (!taskId) return null;
    const inserted = await db
      .insert(Judgments)
      .values({ id: nanoid(), taskId, evaluatorEmail: email, payload: {} })
      .onConflictDoNothing()
      .returning({ id: Judgments.id });
    if (inserted.length > 0) return taskId;
    takenIds.add(taskId);
  }
};

export const releaseTask = async (db: Db, taskId: string, email: string): Promise<void> => {
  await db.insert(TaskReleases).values({ taskId, evaluatorEmail: email }).onConflictDoNothing();
  const [dropped] = await db
    .delete(Judgments)
    .where(and(eq(Judgments.taskId, taskId), eq(Judgments.evaluatorEmail, email), eq(Judgments.draft, true)))
    .returning({ id: Judgments.id });
  if (dropped) {
    await db.delete(JudgmentItems).where(eq(JudgmentItems.judgmentId, dropped.id));
  }
};
