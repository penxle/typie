import { and, asc, eq, inArray, ne, notInArray, sql } from 'drizzle-orm';
import { Judgments, ReleasedTasks, Rounds, Tasks } from './db/index.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

const CAP_SLACK = 1;
const STALE_RELEASE_HOURS = 24;

export const softCap = (requiredTotal: number, expectedEvaluators: number): number =>
  Math.ceil(requiredTotal / expectedEvaluators) + CAP_SLACK;

// 평가자별 소프트 캡 — 라운드 config에 expectedEvaluators가 있으면 균등 몫
// (ceil(필요 판정 총합 / 예상 평가자 수)) + 여유 1건까지만 새 태스크를 배정한다.
// 다른 평가자의 마지막 진행(전무하면 라운드 생성 시각)에서 STALE_RELEASE_HOURS가 지나도록
// 라운드가 멈춰 있으면 캡을 해제한다 — 작업 균등(편향 방지)과 완주 견고성의 절충.
// 반환값은 추가 배정 가능 건수(캡 해제 시 Infinity).
export const capRemaining = (input: {
  myConfirmed: number;
  requiredTotal: number;
  expectedEvaluators: number;
  lastOtherProgressEpochSec: number;
  nowEpochSec: number;
}): number => {
  const left = softCap(input.requiredTotal, input.expectedEvaluators) - input.myConfirmed;
  if (left > 0) return left;
  return input.nowEpochSec - input.lastOtherProgressEpochSec >= STALE_RELEASE_HOURS * 3600 ? Infinity : 0;
};

const claimableQuery = (db: Db, email: string) => {
  const mine = db.select({ taskId: Judgments.taskId }).from(Judgments).where(eq(Judgments.evaluatorEmail, email));
  const released = db.select({ taskId: ReleasedTasks.taskId }).from(ReleasedTasks).where(eq(ReleasedTasks.evaluatorEmail, email));

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
        notInArray(Tasks.id, released),
        sql`(${Tasks.requiredJudgments} is null or coalesce(${counts.total}, 0) < ${Tasks.requiredJudgments})`,
      ),
    )
    .orderBy(asc(Tasks.createdAt));
};

type RoundAllowance = { allowance: number; cap: number | null; myConfirmed: number };

const roundAllowances = async (db: Db, email: string, roundIds: string[]): Promise<Map<string, RoundAllowance>> => {
  const allowances = new Map<string, RoundAllowance>();
  if (roundIds.length === 0) return allowances;

  const rounds = await db.select().from(Rounds).where(inArray(Rounds.id, roundIds));
  const nowEpochSec = Math.floor(Date.now() / 1000);

  for (const round of rounds) {
    const expected = (round.config as { expectedEvaluators?: unknown } | null)?.expectedEvaluators;
    if (typeof expected !== 'number' || !Number.isSafeInteger(expected) || expected < 1) {
      allowances.set(round.id, { allowance: Infinity, cap: null, myConfirmed: 0 });
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

    const myConfirmed = mine?.n ?? 0;
    const requiredTotal = required?.n ?? 0;
    const allowance = capRemaining({
      myConfirmed,
      requiredTotal,
      expectedEvaluators: expected,
      lastOtherProgressEpochSec: lastOther?.t ?? Math.floor(round.createdAt.getTime() / 1000),
      nowEpochSec,
    });
    allowances.set(round.id, { allowance, cap: softCap(requiredTotal, expected), myConfirmed });
  }

  return allowances;
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

  const allowances = await roundAllowances(db, email, [...new Set(candidates.map((c) => c.roundId))]);
  return candidates.find((c) => (allowances.get(c.roundId)?.allowance ?? 0) > 0)?.id ?? null;
};

export type ClaimableSummary = {
  remaining: number;
  quota: { limit: number; used: number } | null;
};

// remaining은 캡을 반영한 "지금 실제로 받을 수 있는" 건수다. quota는 캡이 걸린 라운드가
// 있을 때만 채워진다(복수 라운드면 합산) — 남은 태스크가 아무리 많아도 개인에게는
// 한도까지만 배정된다는 사실을 화면에 그대로 보여주기 위한 값.
export const claimableSummary = async (db: Db, email: string): Promise<ClaimableSummary> => {
  if (await hasOpenDraft(db, email)) return { remaining: 0, quota: null };

  const candidates = await claimableQuery(db, email);
  if (candidates.length === 0) return { remaining: 0, quota: null };

  const allowances = await roundAllowances(db, email, [...new Set(candidates.map((c) => c.roundId))]);
  let remaining = 0;
  const capped = { limit: 0, used: 0, present: false };
  for (const [roundId, info] of allowances) {
    const count = candidates.filter((c) => c.roundId === roundId).length;
    remaining += Math.min(count, info.allowance);
    if (info.cap !== null) {
      capped.limit += info.cap;
      capped.used += info.myConfirmed;
      capped.present = true;
    }
  }
  return { remaining, quota: capped.present ? { limit: capped.limit, used: capped.used } : null };
};
