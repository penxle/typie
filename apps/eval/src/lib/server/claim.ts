import { and, asc, eq, inArray, ne, notInArray, sql } from 'drizzle-orm';
import { Judgments, Rounds, Tasks } from './db/index.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

const CAP_SLACK = 1;
const STALE_RELEASE_HOURS = 24;

// 평가자별 소프트 캡 — 라운드 config에 expectedEvaluators가 있으면 균등 몫
// (ceil(필요 판정 총합 / 예상 평가자 수)) + 여유 1건까지만 새 태스크를 배정한다.
// 다른 평가자의 마지막 진행(전무하면 라운드 생성 시각)에서 STALE_RELEASE_HOURS가 지나도록
// 라운드가 멈춰 있으면 캡을 해제한다 — 작업 균등(편향 방지)과 완주 견고성의 절충.
export const capAllowsMore = (input: {
  myConfirmed: number;
  requiredTotal: number;
  expectedEvaluators: number;
  lastOtherProgressEpochSec: number;
  nowEpochSec: number;
}): boolean => {
  const cap = Math.ceil(input.requiredTotal / input.expectedEvaluators) + CAP_SLACK;
  if (input.myConfirmed < cap) return true;
  return input.nowEpochSec - input.lastOtherProgressEpochSec >= STALE_RELEASE_HOURS * 3600;
};

const claimableQuery = (db: Db, email: string) => {
  const mine = db.select({ taskId: Judgments.taskId }).from(Judgments).where(eq(Judgments.evaluatorEmail, email));

  const counts = db
    .select({ taskId: Judgments.taskId, total: sql<number>`count(*)`.as('total') })
    .from(Judgments)
    .groupBy(Judgments.taskId)
    .as('counts');

  return db
    .select({ id: Tasks.id, roundId: Tasks.roundId })
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

const allowedRounds = async (db: Db, email: string, roundIds: string[]): Promise<Set<string>> => {
  const allowed = new Set<string>();
  if (roundIds.length === 0) return allowed;

  const rounds = await db.select().from(Rounds).where(inArray(Rounds.id, roundIds));
  const nowEpochSec = Math.floor(Date.now() / 1000);

  for (const round of rounds) {
    const expected = (round.config as { expectedEvaluators?: unknown } | null)?.expectedEvaluators;
    if (typeof expected !== 'number' || !Number.isSafeInteger(expected) || expected < 1) {
      allowed.add(round.id);
      continue;
    }

    const roundTaskIds = db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, round.id));
    const [required] = await db
      .select({ n: sql<number>`coalesce(sum(coalesce(${Tasks.requiredJudgments}, 1)), 0)` })
      .from(Tasks)
      .where(eq(Tasks.roundId, round.id));
    const [mine] = await db
      .select({ n: sql<number>`count(*)` })
      .from(Judgments)
      .where(and(eq(Judgments.evaluatorEmail, email), eq(Judgments.draft, false), inArray(Judgments.taskId, roundTaskIds)));
    const [lastOther] = await db
      .select({ t: sql<number | null>`max(${Judgments.updatedAt})` })
      .from(Judgments)
      .where(and(ne(Judgments.evaluatorEmail, email), eq(Judgments.draft, false), inArray(Judgments.taskId, roundTaskIds)));

    const capInput = {
      myConfirmed: mine?.n ?? 0,
      requiredTotal: required?.n ?? 0,
      expectedEvaluators: expected,
      lastOtherProgressEpochSec: lastOther?.t ?? Math.floor(round.createdAt.getTime() / 1000),
      nowEpochSec,
    };
    if (capAllowsMore(capInput)) {
      allowed.add(round.id);
    }
  }

  return allowed;
};

const hasOpenDraft = async (db: Db, email: string): Promise<boolean> => {
  const [openDraft] = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .where(and(eq(Judgments.evaluatorEmail, email), eq(Judgments.draft, true)))
    .limit(1);
  return !!openDraft;
};

export const claimNextTask = async (db: Db, email: string): Promise<string | null> => {
  if (await hasOpenDraft(db, email)) return null;

  const candidates = await claimableQuery(db, email);
  if (candidates.length === 0) return null;

  const allowed = await allowedRounds(db, email, [...new Set(candidates.map((c) => c.roundId))]);
  return candidates.find((c) => allowed.has(c.roundId))?.id ?? null;
};

export const countClaimable = async (db: Db, email: string): Promise<number> => {
  if (await hasOpenDraft(db, email)) return 0;

  const candidates = await claimableQuery(db, email);
  if (candidates.length === 0) return 0;

  const allowed = await allowedRounds(db, email, [...new Set(candidates.map((c) => c.roundId))]);
  return candidates.filter((c) => allowed.has(c.roundId)).length;
};
