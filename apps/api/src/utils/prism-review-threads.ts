import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import dayjs from 'dayjs';
import { and, asc, desc, eq, inArray } from 'drizzle-orm';
import {
  db,
  first,
  firstOrThrow,
  PrismReviewDocumentVersions,
  PrismReviewRounds,
  PrismReviewThreadComments,
  PrismReviewThreads,
  PrismReviewThreadSeats,
  PrismWorkflows,
} from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { PRISM_USER_ID } from '#/utils/system-actor.ts';
import { readMergedGraph } from './changeset.ts';
import { resolveOutcomeAnchors } from './prism-review-anchors.ts';
import { aiCommentId, outcomeAnchorSites, planProjection, unresolvedOutcomeAnchors } from './prism-review-core.ts';
import { wasmThread } from './wasm-thread.ts';
import type { PrismWorkflowState } from '@typie/lib/enums';
import type { ResolvedAnchor, ReviewOutcome } from '@typie/prism';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';

const log = logger.getChild('prism-review');

// settle 트랜잭션이 wasm 풀 대기에 묶이는 시간의 상한 — 넘기면 실패 정책으로 앵커 없이 착지한다
const CAPTURE_TIMEOUT_MS = 60_000;

export type Seat = { roundId: string; issueIndex: number; anchors: ResolvedAnchor[] };
export type LineageRound = {
  id: string;
  round: number;
  closedAt: Dayjs | null;
  result: ReviewOutcome | null;
  workflowState: PrismWorkflowState | null;
};
type ThreadComment = typeof PrismReviewThreadComments.$inferSelect;

// 요청(또는 구독 소켓) 단위 메모 — 스코프 객체는 GraphQL ctx다
type Memo = {
  seats: Map<string, Promise<Map<string, Seat>>>;
  views: Map<string, string>;
  comments: Map<string, Promise<ThreadComment[]>>;
  lineages: Map<string, Promise<LineageRound[]>>;
  latest: Map<string, Promise<Seat | null>>;
};

const memos = new WeakMap<object, Memo>();

const memoOf = (scope: object): Memo => {
  let memo = memos.get(scope);
  if (!memo) {
    memo = { seats: new Map(), views: new Map(), comments: new Map(), lineages: new Map(), latest: new Map() };
    memos.set(scope, memo);
  }

  return memo;
};

// 구독 ctx는 소켓만큼 오래 산다 — 요청 단위로 접으려고 만든 메모가 거기서는 첫 이벤트의 값을 영구히 붙든다.
export const clearRoundMemos = (scope: object): void => {
  memos.delete(scope);
};

// 계보의 회차 목록 — 회차수·잠금·'신규' 판별이 모두 이걸 본다. 스레드마다 다시 읽으면 N+1이라 요청 안에서 한 번만 읽는다.
export const lineageRounds = (scope: object, lineageId: string): Promise<LineageRound[]> => {
  const memo = memoOf(scope);
  const cached = memo.lineages.get(lineageId);
  if (cached) return cached;

  const loading = db
    .select({
      id: PrismReviewRounds.id,
      round: PrismReviewRounds.round,
      closedAt: PrismReviewRounds.closedAt,
      result: PrismReviewRounds.result,
      workflowState: PrismWorkflows.state,
    })
    .from(PrismReviewRounds)
    .leftJoin(PrismWorkflows, eq(PrismWorkflows.id, PrismReviewRounds.workflowId))
    .where(eq(PrismReviewRounds.lineageId, lineageId))
    .orderBy(desc(PrismReviewRounds.round))
    .then((rows) => rows);

  memo.lineages.set(lineageId, loading);
  return loading;
};

export const roundSeats = (scope: object, roundId: string): Promise<Map<string, Seat>> => {
  const memo = memoOf(scope);
  const cached = memo.seats.get(roundId);
  if (cached) return cached;

  const loading = db
    .select({
      threadId: PrismReviewThreadSeats.threadId,
      issueIndex: PrismReviewThreadSeats.issueIndex,
      anchors: PrismReviewThreadSeats.anchors,
    })
    .from(PrismReviewThreadSeats)
    .where(eq(PrismReviewThreadSeats.roundId, roundId))
    .then((rows) => new Map(rows.map((row) => [row.threadId, { roundId, issueIndex: row.issueIndex, anchors: row.anchors }])));

  memo.seats.set(roundId, loading);
  return loading;
};

// 회차 리졸버가 "이 스레드는 이 회차의 눈으로 본다"를 기록하고, 스레드 필드가 그 회차의 좌석·판본으로 답한다.
// 기록이 없으면(뮤테이션 반환 등) 마지막 좌석이 답이다
export const recordView = (scope: object, roundId: string, threadIds: readonly string[]): void => {
  const memo = memoOf(scope);
  for (const id of threadIds) memo.views.set(id, roundId);
};

export const viewedRoundOf = (scope: object, threadId: string): string | null => memoOf(scope).views.get(threadId) ?? null;

export const latestSeat = async (threadId: string): Promise<Seat | null> =>
  db
    .select({
      roundId: PrismReviewThreadSeats.roundId,
      issueIndex: PrismReviewThreadSeats.issueIndex,
      anchors: PrismReviewThreadSeats.anchors,
    })
    .from(PrismReviewThreadSeats)
    .innerJoin(PrismReviewRounds, eq(PrismReviewRounds.id, PrismReviewThreadSeats.roundId))
    .where(eq(PrismReviewThreadSeats.threadId, threadId))
    .orderBy(desc(PrismReviewRounds.round))
    .limit(1)
    .then(first)
    .then((row) => row ?? null);

// 뷰 회차의 좌석이 있으면 그것, 없으면(정리된 스레드) 마지막 좌석.
// 폴백은 한 스레드의 issueIndex·anchors·quote가 저마다 다시 치므로 요청 안에서 한 번만 읽는다.
export const viewSeat = async (
  scope: object,
  threadId: string,
  roundId: string | null = viewedRoundOf(scope, threadId),
): Promise<Seat | null> => {
  if (roundId !== null) {
    const seats = await roundSeats(scope, roundId);
    const seat = seats.get(threadId);
    if (seat) return seat;
  }

  const memo = memoOf(scope);
  const cached = memo.latest.get(threadId);
  if (cached) return cached;

  const loading = latestSeat(threadId);
  memo.latest.set(threadId, loading);
  return loading;
};

export const threadComments = (scope: object, threadId: string): Promise<ThreadComment[]> => {
  const memo = memoOf(scope);
  const cached = memo.comments.get(threadId);
  if (cached) return cached;

  const loading = db
    .select()
    .from(PrismReviewThreadComments)
    .where(eq(PrismReviewThreadComments.threadId, threadId))
    .orderBy(asc(PrismReviewThreadComments.createdAt))
    .then((rows) => rows);

  memo.comments.set(threadId, loading);
  return loading;
};

// 회차 스레드를 한꺼번에 펼칠 때 댓글을 한 번에 읽어 스레드별 메모를 미리 채운다
export const preloadThreadComments = async (scope: object, threadIds: readonly string[]): Promise<void> => {
  if (threadIds.length === 0) return;

  const memo = memoOf(scope);
  const rows = await db
    .select()
    .from(PrismReviewThreadComments)
    .where(inArray(PrismReviewThreadComments.threadId, threadIds))
    .orderBy(asc(PrismReviewThreadComments.createdAt));

  for (const id of threadIds) memo.comments.set(id, Promise.resolve(rows.filter((row) => row.threadId === id)));
};

// 발행은 호출자 몫 — settle 경로는 트랜잭션 안이다. 사영은 회차당 한 번이다: projectedAt이 찍힌 회차는 건드리지 않고,
// 판정은 행 잠금 뒤의 값으로 한다(동시 진입·중복 settle이 두 번 사영·발행하지 않게). 그 안에서 좌석 unique·OPEN 조건부 갱신·
// 결정적 코멘트 id가 중복을 막는다. 반환은 알릴 것이 있었을 때만 — 앉힐 지적·처분이 있었거나 총평 앵커를 처음 굳혔을 때
export const projectRoundThreads = async (executor: Database | Transaction, roundId: string): Promise<{ documentId: string } | null> => {
  const round = await executor
    .select({
      id: PrismReviewRounds.id,
      documentId: PrismReviewRounds.documentId,
      lineageId: PrismReviewRounds.lineageId,
      result: PrismReviewRounds.result,
      conclusionAnchors: PrismReviewRounds.conclusionAnchors,
      projectedAt: PrismReviewRounds.projectedAt,
      content: PrismReviewDocumentVersions.content,
      heads: PrismReviewDocumentVersions.heads,
    })
    .from(PrismReviewRounds)
    .innerJoin(PrismReviewDocumentVersions, eq(PrismReviewDocumentVersions.id, PrismReviewRounds.documentVersionId))
    .where(eq(PrismReviewRounds.id, roundId))
    .then(first);
  if (!round || round.result === null || round.projectedAt !== null) return null;

  const plan = planProjection(round.result);
  const planEmpty = plan.fresh.length === 0 && plan.carried.length === 0 && plan.dispositions.length === 0;
  // 총평 앵커는 좌석과 무관하게 회차당 한 번만 굳힌다 — 지적 없는 회차도 강점은 있고, 한 번 앉은 값은 재사영이 지우지 않는다
  const needsConclusion = round.conclusionAnchors === null && outcomeAnchorSites(round.result).length > 0;

  // 원고 크기 비례 작업은 행 잠금 앞에 — 잠금 보유 시간에 해석·캡처 시간이 들어가지 않게 한다.
  // 좌석에 앉힐 지적이 없고 총평도 이미 굳었으면 wasm을 부르지 않는다.
  const anchors =
    needsConclusion || plan.fresh.length > 0 || plan.carried.length > 0
      ? await resolveOutcomeAnchors(
          round.result,
          { content: round.content, heads: round.heads },
          {
            readGraph: () => readMergedGraph(round.documentId),
            capture: (graph, heads, expectedText, ranges) =>
              wasmThread.captureProseAnchors(graph, heads, expectedText, ranges, CAPTURE_TIMEOUT_MS).then(({ result }) => result),
            report: (failure) => {
              if (failure.kind === 'text_mismatch') {
                log.warn('review anchors dropped: snapshot text differs at heads {roundId}', { roundId: round.id });
                Sentry.captureMessage(`prism review anchors: snapshot text mismatch ${round.id}`, {
                  level: 'warning',
                  extra: { roundId: round.id },
                });
              } else {
                log.warn('review anchors dropped: capture failed {roundId} {*}', { roundId: round.id, error: failure.error });
                Sentry.captureException(failure.error, { extra: { roundId: round.id } });
              }
            },
          },
        )
      : unresolvedOutcomeAnchors(round.result);

  const locked = await executor
    .select({ projectedAt: PrismReviewRounds.projectedAt, conclusionAnchors: PrismReviewRounds.conclusionAnchors })
    .from(PrismReviewRounds)
    .where(eq(PrismReviewRounds.id, round.id))
    .for('update')
    .then(firstOrThrow);
  if (locked.projectedAt !== null) return null;

  await executor
    .update(PrismReviewRounds)
    .set({ projectedAt: dayjs(), ...(locked.conclusionAnchors === null && { conclusionAnchors: anchors.conclusion }) })
    .where(eq(PrismReviewRounds.id, round.id));

  if (planEmpty) return needsConclusion ? { documentId: round.documentId } : null;

  const seated = await executor
    .select({ issueIndex: PrismReviewThreadSeats.issueIndex })
    .from(PrismReviewThreadSeats)
    .where(eq(PrismReviewThreadSeats.roundId, round.id))
    .then((rows) => new Set(rows.map((row) => row.issueIndex)));

  for (const fresh of plan.fresh) {
    if (seated.has(fresh.issueIndex)) continue;

    const thread = await executor
      .insert(PrismReviewThreads)
      .values({
        documentId: round.documentId,
        lineageId: round.lineageId,
        bornRoundId: round.id,
        issueId: fresh.issueId,
        trait: fresh.trait,
        pass: fresh.pass,
        body: fresh.body,
      })
      .returning({ id: PrismReviewThreads.id })
      .then(firstOrThrow);

    await executor
      .insert(PrismReviewThreadSeats)
      .values({ threadId: thread.id, roundId: round.id, issueIndex: fresh.issueIndex, anchors: anchors.issues[fresh.issueIndex] ?? [] })
      .onConflictDoNothing({ target: [PrismReviewThreadSeats.roundId, PrismReviewThreadSeats.issueIndex] });
  }

  const lineageThreads = await executor
    .select({ id: PrismReviewThreads.id, state: PrismReviewThreads.state })
    .from(PrismReviewThreads)
    .where(eq(PrismReviewThreads.lineageId, round.lineageId))
    .then((rows) => new Map(rows.map((row) => [row.id, row.state])));

  for (const carried of plan.carried) {
    const state = lineageThreads.get(carried.threadId);
    if (state === undefined) {
      log.warn('carried thread not in lineage: {threadId} ({roundId})', { threadId: carried.threadId, roundId: round.id });
      continue;
    }
    if (state !== 'OPEN') continue;

    await executor
      .insert(PrismReviewThreadSeats)
      .values({
        threadId: carried.threadId,
        roundId: round.id,
        issueIndex: carried.issueIndex,
        anchors: anchors.issues[carried.issueIndex] ?? [],
      })
      .onConflictDoNothing({ target: [PrismReviewThreadSeats.threadId, PrismReviewThreadSeats.roundId] });
  }

  for (const disposition of plan.dispositions) {
    const state = lineageThreads.get(disposition.threadId);
    if (state === undefined) {
      log.warn('disposed thread not in lineage: {threadId} ({roundId})', { threadId: disposition.threadId, roundId: round.id });
      continue;
    }
    if (state === 'CLOSED') continue;

    if (disposition.verdict !== 'kept') {
      await executor
        .update(PrismReviewThreads)
        .set({ state: disposition.verdict === 'resolved' ? 'RESOLVED' : 'WITHDRAWN', stateChangedAt: dayjs(), settledRoundId: round.id })
        .where(and(eq(PrismReviewThreads.id, disposition.threadId), eq(PrismReviewThreads.state, 'OPEN')));
    }

    if (disposition.comment === null || disposition.comment.trim().length === 0) continue;

    await executor
      .insert(PrismReviewThreadComments)
      .values({
        id: aiCommentId(disposition.threadId, round.id),
        threadId: disposition.threadId,
        author: 'AI',
        userId: PRISM_USER_ID,
        body: disposition.comment,
      })
      .onConflictDoNothing({ target: [PrismReviewThreadComments.id] });
  }

  return { documentId: round.documentId };
};

// 배포 전에 끝난 회차(결과만 있고 사영 흔적이 없는 행)를 첫 조회가 메운다. settle은 결과 기록과 사영이 한 트랜잭션이라
// 사영 중에 죽어도 결과 없는 회차로 되돌아간다 — 여기서 메울 것은 흔적 없는 회차뿐이다.
// 바깥 읽기는 지름길이고 확정 판정은 projectRoundThreads가 잠금 뒤에 다시 한다
export const ensureRoundThreads = async (roundId: string): Promise<void> => {
  const round = await db
    .select({ projectedAt: PrismReviewRounds.projectedAt })
    .from(PrismReviewRounds)
    .where(eq(PrismReviewRounds.id, roundId))
    .then(first);
  if (!round || round.projectedAt !== null) return;

  const projected = await db.transaction((tx) => projectRoundThreads(tx, roundId));
  if (projected !== null) pubsub.publish('prism:review', projected.documentId, { roundId });
};

// 뮤테이션 발행용 — 스레드가 지금 앉은(마지막 좌석) 회차
export const publishThread = async (thread: { id: string; documentId: string }): Promise<void> => {
  const seat = await latestSeat(thread.id);
  if (seat !== null) pubsub.publish('prism:review', thread.documentId, { roundId: seat.roundId });
};
