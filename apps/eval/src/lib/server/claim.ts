import { and, asc, eq, gte, inArray, isNull, lt, notInArray, or, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { EvaluatorConsents, Judgments, ReleasedTasks, Rounds, Tasks } from './db/index.ts';
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

// 늦게 합류한 평가자의 최소 몫 예약 — 아직 최소 몫(floor(필요 총합 / 동의 인원))을 못 채운
// 다른 평가자들의 부족분만큼을 남은 작업에서 떼어 둔다. 자동 만료는 없다: 예약으로 라운드가
// 멈추면 관리자가 직접 참여를 요청하거나 동의 명단에서 제거해 해소한다(정책적 해결, 오너 결정).
export const reserveAllowance = (input: {
  requiredTotal: number;
  effectiveDone: number;
  participants: number;
  othersConfirmed: number[];
}): number => {
  if (input.participants <= 1) return Infinity;
  const minShare = Math.floor(input.requiredTotal / input.participants);
  const reserved = input.othersConfirmed.reduce((sum, n) => sum + Math.max(0, minShare - n), 0);
  return Math.max(0, input.requiredTotal - input.effectiveDone - reserved);
};

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

// 참여자 = 동의 명단 − 어드민. 최소 몫·예약 계산은 실제 평가 인력만으로 한다.
const participantEmails = async (db: Db, adminEmails: string): Promise<string[]> => {
  const admins = new Set(
    adminEmails
      .split(',')
      .map((e) => e.trim())
      .filter((e) => e.length > 0),
  );
  const consents = await db.select({ email: EvaluatorConsents.email }).from(EvaluatorConsents);
  return consents.map((c) => c.email).filter((e) => !admins.has(e));
};

// 라운드가 필요 수를 이미 채웠어도 최소 몫을 못 채운 평가자에게는 몫만큼 추가 판정을 연다 —
// 뒤늦은 합류자의 의사도 데이터에 반영한다(태스크별 전 판정 평균 집계가 추가 판정을 흡수한다).
// 커버리지가 가장 낮은 문서부터 배정해 추가 판정의 정보 가치를 최대화한다.
const catchUp = async (db: Db, email: string, participants: string[]): Promise<{ count: number; taskId: string | null }> => {
  if (!participants.includes(email)) return { count: 0, taskId: null };

  const releasedRows = await db.select({ taskId: ReleasedTasks.taskId }).from(ReleasedTasks).where(eq(ReleasedTasks.evaluatorEmail, email));
  const releasedSet = new Set(releasedRows.map((r) => r.taskId));

  const rounds = await db.select({ id: Rounds.id }).from(Rounds).orderBy(asc(Rounds.createdAt));
  let count = 0;
  let taskId: string | null = null;
  for (const round of rounds) {
    const roundTasks = await db
      .select({ id: Tasks.id, requiredJudgments: Tasks.requiredJudgments, createdAt: Tasks.createdAt })
      .from(Tasks)
      .where(eq(Tasks.roundId, round.id));
    if (roundTasks.length === 0) continue;

    const judgments = await db
      .select({ taskId: Judgments.taskId, evaluatorEmail: Judgments.evaluatorEmail, draft: Judgments.draft })
      .from(Judgments)
      .where(inArray(Judgments.taskId, db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, round.id))));

    const requiredTotal = roundTasks.reduce((sum, t) => sum + (t.requiredJudgments ?? 1), 0);
    const minShare = Math.floor(requiredTotal / participants.length);
    const myConfirmed = judgments.filter((j) => j.evaluatorEmail === email && !j.draft).length;
    if (myConfirmed >= minShare) continue;

    const myTaskIds = new Set(judgments.filter((j) => j.evaluatorEmail === email).map((j) => j.taskId));
    const coverage = new Map<string, number>();
    for (const judgment of judgments) {
      if (judgment.draft) continue;
      coverage.set(judgment.taskId, (coverage.get(judgment.taskId) ?? 0) + 1);
    }
    const available = roundTasks
      .filter((t) => !myTaskIds.has(t.id) && !releasedSet.has(t.id))
      .toSorted((a, b) => (coverage.get(a.id) ?? 0) - (coverage.get(b.id) ?? 0) || a.createdAt.getTime() - b.createdAt.getTime());
    if (available.length === 0) continue;

    count += Math.min(minShare - myConfirmed, available.length);
    taskId ??= available[0].id;
  }
  return { count, taskId };
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

const roundAllowances = async (db: Db, email: string, roundIds: string[], participants: string[]): Promise<Map<string, RoundAllowance>> => {
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
    const capAllowance = capRemaining({
      myConfirmed,
      requiredTotal: capBase,
      expectedEvaluators: expected,
      lastOtherProgressEpochSec: lastOther ?? Math.floor(round.createdAt.getTime() / 1000),
      nowEpochSec,
    });

    const confirmedByEmail = new Map<string, number>();
    for (const row of confirmedRows) {
      confirmedByEmail.set(row.evaluatorEmail, (confirmedByEmail.get(row.evaluatorEmail) ?? 0) + 1);
    }
    const effectiveDone = roundTasks.reduce((sum, t) => sum + Math.min(confirmedByTask.get(t.id) ?? 0, t.requiredJudgments ?? 1), 0);
    const reserved = reserveAllowance({
      requiredTotal,
      effectiveDone,
      participants: participants.length,
      othersConfirmed: participants.filter((p) => p !== email).map((p) => confirmedByEmail.get(p) ?? 0),
    });

    allowances.set(round.id, { allowance: Math.min(capAllowance, reserved), cap: softCap(capBase, expected), myConfirmed });
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

export const claimNextTask = async (db: Db, email: string, adminEmails = ''): Promise<string | null> => {
  await reclaimExpiredReservations(db);
  if (await hasOpenDraft(db, email)) return null;

  const participants = await participantEmails(db, adminEmails);

  const candidates = await claimableQuery(db, email);
  let taskId: string | null = null;
  if (candidates.length > 0) {
    const allowances = await roundAllowances(db, email, [...new Set(candidates.map((c) => c.roundId))], participants);
    taskId = candidates.find((c) => (allowances.get(c.roundId)?.allowance ?? 0) > 0)?.id ?? null;
  }
  if (!taskId) {
    const caughtUp = await catchUp(db, email, participants);
    taskId = caughtUp.taskId;
  }

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
  // potential은 열린 draft를 끝냈다고 가정했을 때 받을 수 있는 수 — 개인 진행 분모(완료+작성 중+potential)
  // 표시용이다. remaining은 미완료 draft 차단 규칙이 적용된 "지금 당장 받을 수 있는 수"(버튼 게이트용).
  potential: number;
  quota: { limit: number; used: number } | null;
};

// remaining은 캡을 반영한 "지금 실제로 받을 수 있는" 건수다. quota는 캡이 걸린 라운드가
// 있을 때만 채워진다(복수 라운드면 합산) — 남은 태스크가 아무리 많아도 개인에게는
// 한도까지만 배정된다는 사실을 화면에 그대로 보여주기 위한 값.
export const claimableSummary = async (db: Db, email: string, adminEmails = ''): Promise<ClaimableSummary> => {
  await reclaimExpiredReservations(db);

  const participants = await participantEmails(db, adminEmails);

  const candidates = await claimableQuery(db, email);
  let potential = 0;
  const capped = { limit: 0, used: 0, present: false };
  if (candidates.length > 0) {
    const allowances = await roundAllowances(db, email, [...new Set(candidates.map((c) => c.roundId))], participants);
    for (const [roundId, info] of allowances) {
      const count = candidates.filter((c) => c.roundId === roundId).length;
      potential += Math.min(count, info.allowance);
      if (info.cap !== null) {
        capped.limit += info.cap;
        capped.used += info.myConfirmed;
        capped.present = true;
      }
    }
  }
  if (potential === 0) {
    const caughtUp = await catchUp(db, email, participants);
    potential = caughtUp.count;
  }

  const blocked = await hasOpenDraft(db, email);
  return { remaining: blocked ? 0 : potential, potential, quota: capped.present ? { limit: capped.limit, used: capped.used } : null };
};
