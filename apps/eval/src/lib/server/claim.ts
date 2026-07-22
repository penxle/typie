import { and, asc, eq, gte, inArray, isNull, lt, notInArray, or, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { Judgments, ReleasedTasks, Rounds, Tasks } from './db/index.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

const CAP_SLACK = 1;
const STALE_RELEASE_HOURS = 24;

// 배정 예약과 회수 — claim 시 빈 draft를 만들어 중복 배정을 막되, 파이프라인이 영구히 멈추는 것이
// 중복 평가보다 나쁘다는 운영 판단에 따라 두 단계로 회수한다:
// ① 입력이 전혀 없는 예약은 RESERVATION_TTL_HOURS 뒤 삭제 — 태스크와 평가자 양쪽을 즉시 해방.
// ② 입력이 있는 draft도 STALE_DRAFT_HOURS 정체 시 배정 차단 효력만 상실 — 작업물은 보존되고
//    본인은 언제든 이어서 제출할 수 있으며, 그 사이 타인이 배정받으면 잉여 판정 1건으로 그친다.
const RESERVATION_TTL_HOURS = 2;
const STALE_DRAFT_HOURS = 24;

const reclaimExpiredReservations = async (db: Db): Promise<void> => {
  const cutoff = new Date(Date.now() - RESERVATION_TTL_HOURS * 3600 * 1000);
  await db
    .delete(Judgments)
    .where(
      and(
        eq(Judgments.draft, true),
        isNull(Judgments.result),
        sql`(${Judgments.comment} is null or ${Judgments.comment} = '')`,
        sql`(${Judgments.feedbackLabels} is null or ${Judgments.feedbackLabels} = '{}')`,
        lt(Judgments.updatedAt, cutoff),
      ),
    );
};

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

  const staleDraftCutoff = new Date(Date.now() - STALE_DRAFT_HOURS * 3600 * 1000);
  const counts = db
    .select({ taskId: Judgments.taskId, total: sql<number>`count(*)`.as('total') })
    .from(Judgments)
    .where(or(eq(Judgments.draft, false), gte(Judgments.updatedAt, staleDraftCutoff)))
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
    const roundTasks = await db
      .select({ id: Tasks.id, requiredJudgments: Tasks.requiredJudgments })
      .from(Tasks)
      .where(eq(Tasks.roundId, round.id));
    const confirmedRows = await db
      .select({
        id: Judgments.id,
        taskId: Judgments.taskId,
        evaluatorEmail: Judgments.evaluatorEmail,
        createdAt: Judgments.createdAt,
        updatedAt: Judgments.updatedAt,
      })
      .from(Judgments)
      .where(and(eq(Judgments.draft, false), inArray(Judgments.taskId, roundTaskIds)));

    const requiredTotal = roundTasks.reduce((sum, t) => sum + (t.requiredJudgments ?? 1), 0);
    // 개인 캡에는 잉여 포함 모든 확정 판정을 계상한다 — 평가자가 들인 노동은 전부 크레딧이다.
    // 대신 캡 분자에 관측된 잉여를 더해 라운드 총 용량(캡×인원)이 필요 총합 아래로 떨어지지
    // 않게 한다. 잉여를 캡에서 빼는 방식은 "당신 판정은 무효" 메시지가 되어 채택하지 않았다.
    const confirmedByTask = new Map<string, number>();
    for (const row of confirmedRows) {
      confirmedByTask.set(row.taskId, (confirmedByTask.get(row.taskId) ?? 0) + 1);
    }
    const surplusTotal = roundTasks.reduce((sum, t) => sum + Math.max(0, (confirmedByTask.get(t.id) ?? 0) - (t.requiredJudgments ?? 1)), 0);
    const myConfirmed = confirmedRows.filter((row) => row.evaluatorEmail === email).length;
    const lastOther = confirmedRows
      .filter((row) => row.evaluatorEmail !== email)
      .reduce<number | null>((max, row) => {
        const t = Math.floor(row.updatedAt.getTime() / 1000);
        return max === null || t > max ? t : max;
      }, null);

    const capBase = requiredTotal + surplusTotal;
    const allowance = capRemaining({
      myConfirmed,
      requiredTotal: capBase,
      expectedEvaluators: expected,
      lastOtherProgressEpochSec: lastOther ?? Math.floor(round.createdAt.getTime() / 1000),
      nowEpochSec,
    });
    allowances.set(round.id, { allowance, cap: softCap(capBase, expected), myConfirmed });
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
  await reclaimExpiredReservations(db);
  if (await hasOpenDraft(db, email)) return null;

  const candidates = await claimableQuery(db, email);
  if (candidates.length === 0) return null;

  const allowances = await roundAllowances(db, email, [...new Set(candidates.map((c) => c.roundId))]);
  const taskId = candidates.find((c) => (allowances.get(c.roundId)?.allowance ?? 0) > 0)?.id ?? null;

  if (taskId) {
    // 배정 즉시 빈 draft로 예약 — 이후 다른 평가자의 counts에 잡혀 중복 배정이 차단된다.
    await db
      .insert(Judgments)
      .values({ id: nanoid(), taskId, evaluatorEmail: email })
      .onConflictDoNothing({ target: [Judgments.taskId, Judgments.evaluatorEmail] });
  }

  return taskId;
};

export type ClaimableSummary = {
  remaining: number;
  quota: { limit: number; used: number } | null;
};

// remaining은 캡을 반영한 "지금 실제로 받을 수 있는" 건수다. quota는 캡이 걸린 라운드가
// 있을 때만 채워진다(복수 라운드면 합산) — 남은 태스크가 아무리 많아도 개인에게는
// 한도까지만 배정된다는 사실을 화면에 그대로 보여주기 위한 값.
export const claimableSummary = async (db: Db, email: string): Promise<ClaimableSummary> => {
  await reclaimExpiredReservations(db);
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
