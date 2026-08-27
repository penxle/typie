import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import { quoteReviewCredits, ReviewOutcomeEnvelopeSchema } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, desc, eq, inArray, isNull } from 'drizzle-orm';
import { PgTransaction } from 'drizzle-orm/pg-core';
import {
  db,
  Documents,
  first,
  firstOrThrow,
  PrismReviewDocumentVersions,
  PrismReviewLineages,
  PrismReviewRounds,
  PrismReviewThreadComments,
  PrismReviewThreads,
  PrismReviewThreadSeats,
  PrismSessions,
  PrismWorkflows,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { activeRun, prism } from '#/external/prism.ts';
import { pubsub } from '#/pubsub.ts';
import { assertDocumentPermission } from './permission.ts';
import { prismReviewSeeds } from './prism-catalog.ts';
import { chargePrismCredit, lockUserPrismCredit, readPrismCreditBalance, refundPrismReview } from './prism-credit.ts';
import { toMilli } from './prism-credit-core.ts';
import { snapshotManuscript } from './prism-manuscript.ts';
import {
  buildPreviousContext,
  ConfirmInputSchema,
  confirmResult,
  ENUM_TO_TIER,
  lineageLocked,
  manuscriptPath,
  pickVersion,
  seedsPrefix,
  seedUploads,
  summarizeOutcome,
} from './prism-review-core.ts';
import { ensureRoundThreads, projectRoundThreads } from './prism-review-threads.ts';
import type { PrismReviewTier } from '@typie/lib/enums';
import type { PrismReviewTierName, ResolvedAnchor, ReviewOutcome, ReviewSeedMapping } from '@typie/prism';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';
import type { PrismAppHooks, PrismWorkflowRow, WorkflowOutcome } from './prism-apps.ts';
import type { Manuscript } from './prism-manuscript.ts';
import type { PrismToolContext, PrismToolHandler } from './prism-tools.ts';

const log = logger.getChild('prism-review');

const upsertDocumentVersion = async (
  tx: Transaction,
  documentId: string,
  snap: Manuscript,
): Promise<{ id: string; characterCount: number } | null> => {
  const latest = await tx
    .select({
      id: PrismReviewDocumentVersions.id,
      version: PrismReviewDocumentVersions.version,
      content: PrismReviewDocumentVersions.content,
      title: PrismReviewDocumentVersions.title,
      subtitle: PrismReviewDocumentVersions.subtitle,
      characterCount: PrismReviewDocumentVersions.characterCount,
    })
    .from(PrismReviewDocumentVersions)
    .where(eq(PrismReviewDocumentVersions.documentId, documentId))
    .orderBy(desc(PrismReviewDocumentVersions.version))
    .limit(1)
    .then(first);

  const picked = pickVersion(latest ?? null, snap);
  if (latest && picked.reuse) return { id: latest.id, characterCount: latest.characterCount };

  const inserted = await tx
    .insert(PrismReviewDocumentVersions)
    .values({
      documentId,
      version: picked.version,
      title: snap.title,
      subtitle: snap.subtitle,
      content: snap.content,
      characterCount: snap.characterCount,
      heads: snap.heads,
    })
    .onConflictDoNothing({ target: [PrismReviewDocumentVersions.documentId, PrismReviewDocumentVersions.version] })
    .returning({ id: PrismReviewDocumentVersions.id, characterCount: PrismReviewDocumentVersions.characterCount })
    .then(first);

  return inserted ?? null;
};

const refundRound = async (tx: Transaction, roundId: string): Promise<boolean> => {
  const result = await refundPrismReview(tx, { roundId });
  if (!result.applied && result.reason === 'no_charge') log.warn('review refund skipped: no charge for round {roundId}', { roundId });
  return result.applied;
};

const closePendingRounds = async (executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void> => {
  const run = async (tx: Transaction) => {
    const closed = await tx
      .update(PrismReviewRounds)
      .set({ closedAt: dayjs() })
      .where(
        and(
          eq(PrismReviewRounds.sessionId, sessionId),
          eq(PrismReviewRounds.prismRunSeq, runSeq),
          isNull(PrismReviewRounds.workflowId),
          isNull(PrismReviewRounds.closedAt),
        ),
      )
      .returning({ id: PrismReviewRounds.id });

    for (const round of closed) {
      await refundRound(tx, round.id);
    }
  };

  if (executor instanceof PgTransaction) {
    await run(executor as Transaction);
  } else {
    await executor.transaction(run);
  }
};

const createRound = async (input: {
  userId: string;
  sessionId: string;
  documentId: string;
  runSeq: number;
  tier: PrismReviewTier;
  versionId: string;
  characterCount: number;
  lineageId: string;
  baseRoundId: string | null;
}): Promise<{ id: string }> => {
  const amount = toMilli(quoteReviewCredits(ENUM_TO_TIER[input.tier], input.characterCount));

  for (let attempt = 0; attempt < 2; attempt++) {
    const created = await db.transaction(async (tx) => {
      await closePendingRounds(tx, input.sessionId, input.runSeq);

      const latest = await tx
        .select({ round: PrismReviewRounds.round })
        .from(PrismReviewRounds)
        .where(eq(PrismReviewRounds.documentId, input.documentId))
        .orderBy(desc(PrismReviewRounds.round))
        .limit(1)
        .then(first);

      const row = await tx
        .insert(PrismReviewRounds)
        .values({
          documentId: input.documentId,
          round: (latest?.round ?? 0) + 1,
          sessionId: input.sessionId,
          prismRunSeq: input.runSeq,
          tier: input.tier,
          documentVersionId: input.versionId,
          lineageId: input.lineageId,
          baseRoundId: input.baseRoundId,
        })
        .onConflictDoNothing({ target: [PrismReviewRounds.documentId, PrismReviewRounds.round] })
        .returning({ id: PrismReviewRounds.id })
        .then(first);

      if (!row) return null;

      await lockUserPrismCredit(tx, input.userId);
      const balance = await readPrismCreditBalance(tx, input.userId);
      if (balance.total < amount) throw new TypieError({ code: 'prism_credit_insufficient', status: 403 });

      await chargePrismCredit(tx, { userId: input.userId, kind: 'REVIEW_CHARGE', key: row.id, amount });

      return { id: row.id };
    });

    if (created) {
      pubsub.publish('prism:credit', input.userId, {});
      return created;
    }
  }

  throw new TypieError({ code: 'prism_round_conflict', status: 409 });
};

const closeRound = async (roundId: string): Promise<void> => {
  const refunded = await db.transaction(async (tx) => {
    const closed = await tx
      .update(PrismReviewRounds)
      .set({ closedAt: dayjs() })
      .where(and(eq(PrismReviewRounds.id, roundId), isNull(PrismReviewRounds.closedAt)))
      .returning({ id: PrismReviewRounds.id })
      .then(first);

    if (!closed) return null;

    const round = await tx
      .select({ userId: PrismSessions.userId })
      .from(PrismReviewRounds)
      .innerJoin(PrismSessions, eq(PrismSessions.id, PrismReviewRounds.sessionId))
      .where(eq(PrismReviewRounds.id, roundId))
      .then(first);

    await refundRound(tx, roundId);

    return round?.userId ?? null;
  });

  if (refunded) pubsub.publish('prism:credit', refunded, {});
};

export const prepareReviewSnapshot = async ({
  userId,
  documentId,
}: {
  userId: string;
  documentId: string;
}): Promise<{ versionId: string; characterCount: number }> => {
  await assertDocumentPermission({ userId, documentId });
  const snap = await snapshotManuscript(documentId);

  for (let attempt = 0; attempt < 2; attempt++) {
    const version = await db.transaction((tx) => upsertDocumentVersion(tx, documentId, snap));
    if (version) return { versionId: version.id, characterCount: version.characterCount };
  }

  throw new TypieError({ code: 'prism_round_conflict', status: 409 });
};

type BaseRound = {
  id: string;
  createdAt: Dayjs;
  documentVersionId: string;
  title: string | null;
  subtitle: string | null;
  content: string;
  prismWorkflowId: string;
};

const openLineage = async (documentId: string, tier: PrismReviewTier): Promise<{ id: string; base: null }> => {
  const lineage = await db
    .insert(PrismReviewLineages)
    .values({ documentId, tier })
    .returning({ id: PrismReviewLineages.id })
    .then(firstOrThrow);

  return { id: lineage.id, base: null };
};

// base는 계보의 최신 완료·비거부 회차 — 진행 중 회차가 있으면 잇지 못한다(입력을 굳힌 회차 위에 또 굳힐 수 없다)
const continueLineage = async (lineageId: string, documentId: string, tier: PrismReviewTier): Promise<{ id: string; base: BaseRound }> => {
  const lineage = await db.select().from(PrismReviewLineages).where(eq(PrismReviewLineages.id, lineageId)).then(first);
  if (!lineage || lineage.documentId !== documentId || lineage.tier !== tier)
    throw new TypieError({ code: 'invalid_confirm_input', status: 400 });

  const rounds = await db
    .select({
      id: PrismReviewRounds.id,
      createdAt: PrismReviewRounds.createdAt,
      closedAt: PrismReviewRounds.closedAt,
      result: PrismReviewRounds.result,
      documentVersionId: PrismReviewRounds.documentVersionId,
      workflowState: PrismWorkflows.state,
      prismWorkflowId: PrismWorkflows.prismWorkflowId,
      title: PrismReviewDocumentVersions.title,
      subtitle: PrismReviewDocumentVersions.subtitle,
      content: PrismReviewDocumentVersions.content,
    })
    .from(PrismReviewRounds)
    .leftJoin(PrismWorkflows, eq(PrismWorkflows.id, PrismReviewRounds.workflowId))
    .innerJoin(PrismReviewDocumentVersions, eq(PrismReviewDocumentVersions.id, PrismReviewRounds.documentVersionId))
    .where(eq(PrismReviewRounds.lineageId, lineageId))
    .orderBy(desc(PrismReviewRounds.round));

  if (lineageLocked(rounds.map((round) => ({ closedAt: round.closedAt, workflowState: round.workflowState }))))
    throw new TypieError({ code: 'prism_review_running', status: 409 });

  const base = rounds.find(
    (round) =>
      round.workflowState === 'COMPLETED' &&
      round.prismWorkflowId !== null &&
      round.result !== null &&
      summarizeOutcome(round.result).rejection === null,
  );
  if (!base || base.prismWorkflowId === null) throw new TypieError({ code: 'prism_review_no_base', status: 409 });

  return {
    id: lineage.id,
    base: {
      id: base.id,
      createdAt: base.createdAt,
      documentVersionId: base.documentVersionId,
      title: base.title,
      subtitle: base.subtitle,
      content: base.content,
      prismWorkflowId: base.prismWorkflowId,
    },
  };
};

// 회차 행이 선 뒤에 읽는다 — 잠금 이후의 읽기가 이번 회차의 확정 입력이다
const followupMaterials = async (roundId: string, lineageId: string, base: BaseRound, tier: PrismReviewTierName) => {
  // 결과만 있고 사영되지 않은 base(배포 전 종료)를 이으면 지난 리뷰가 통째로 비어 실린다
  await ensureRoundThreads(base.id);

  const threads = await db
    .select()
    .from(PrismReviewThreads)
    .where(eq(PrismReviewThreads.lineageId, lineageId))
    .orderBy(asc(PrismReviewThreads.createdAt));
  const threadIds = threads.map((thread) => thread.id);
  const seats =
    threadIds.length === 0
      ? []
      : await db
          .select({
            threadId: PrismReviewThreadSeats.threadId,
            roundId: PrismReviewThreadSeats.roundId,
            anchors: PrismReviewThreadSeats.anchors,
          })
          .from(PrismReviewThreadSeats)
          .innerJoin(PrismReviewRounds, eq(PrismReviewRounds.id, PrismReviewThreadSeats.roundId))
          .where(inArray(PrismReviewThreadSeats.threadId, threadIds))
          .orderBy(asc(PrismReviewRounds.round));
  const comments =
    threadIds.length === 0
      ? []
      : await db
          .select()
          .from(PrismReviewThreadComments)
          .where(inArray(PrismReviewThreadComments.threadId, threadIds))
          .orderBy(asc(PrismReviewThreadComments.createdAt));

  // base 회차의 좌석이 없으면 가장 높은 회차의 좌석 — latestSeat과 같은 정의다
  const anchorsOf = (threadId: string): ResolvedAnchor[] => {
    const mine = seats.filter((seat) => seat.threadId === threadId);
    return (mine.find((seat) => seat.roundId === base.id) ?? mine.at(-1))?.anchors ?? [];
  };

  const previous = buildPreviousContext({
    base: { title: base.title, subtitle: base.subtitle, versionId: base.documentVersionId, createdAt: base.createdAt.toDate() },
    threads: threads.map((thread) => ({
      id: thread.id,
      pass: thread.pass,
      trait: thread.trait,
      body: thread.body,
      state: thread.state,
      issueId: thread.issueId,
      anchors: anchorsOf(thread.id),
      comments: comments
        .filter((comment) => comment.threadId === thread.id)
        .map((comment) => ({ author: comment.author, body: comment.body, createdAt: comment.createdAt.toDate() })),
    })),
  });

  let seeds: ReviewSeedMapping[];
  try {
    seeds = await prismReviewSeeds(tier);
  } catch (err) {
    log.warn('review seeds catalog unavailable: {*}', { error: err });
    throw new TypieError({ code: 'prism_review_seed_unavailable', status: 502 });
  }

  const contents = new Map<string, string | null>();
  for (const seed of seeds) {
    try {
      contents.set(seed.from, await prism.getWorkflowFile(base.prismWorkflowId, seed.from));
    } catch (err) {
      log.warn('review seed fetch failed: {path} {*}', { path: seed.from, error: err });
      throw new TypieError({ code: 'prism_review_seed_unavailable', status: 502 });
    }
  }

  const uploads = seedUploads(roundId, seeds, contents);
  if ('missing' in uploads) {
    log.warn('review seed missing in base workflow: {path}', { path: uploads.missing });
    throw new TypieError({ code: 'prism_review_seed_unavailable', status: 502 });
  }

  return { previous, uploads };
};

const confirmReview = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ConfirmInputSchema.safeParse(input);
  if (!parsed.success) throw new TypieError({ code: 'invalid_confirm_input', status: 400 });
  if (parsed.data.decision === 'declined') return { decision: 'declined' } as const;

  const { versionId, tier, lineageId } = parsed.data;
  if (!validateDbId(TableCode.PRISM_REVIEW_DOCUMENT_VERSIONS).regex.test(versionId))
    throw new TypieError({ code: 'invalid_confirm_input', status: 400 });
  if (lineageId !== undefined && !validateDbId(TableCode.PRISM_REVIEW_LINEAGES).regex.test(lineageId))
    throw new TypieError({ code: 'invalid_confirm_input', status: 400 });

  const version = await db
    .select({
      id: PrismReviewDocumentVersions.id,
      documentId: PrismReviewDocumentVersions.documentId,
      title: PrismReviewDocumentVersions.title,
      subtitle: PrismReviewDocumentVersions.subtitle,
      content: PrismReviewDocumentVersions.content,
      characterCount: PrismReviewDocumentVersions.characterCount,
    })
    .from(PrismReviewDocumentVersions)
    .where(eq(PrismReviewDocumentVersions.id, versionId))
    .then(first);
  if (!version) throw new TypieError({ code: 'not_found', status: 404 });

  await assertDocumentPermission({ userId: ctx.userId, documentId: version.documentId });

  const running = activeRun(ctx.agent.runs);
  if (!running) throw new TypieError({ code: 'prism_tool_settled', status: 409 });

  const lineage =
    lineageId === undefined ? await openLineage(version.documentId, tier) : await continueLineage(lineageId, version.documentId, tier);

  const round = await createRound({
    userId: ctx.userId,
    sessionId: ctx.session.id,
    documentId: version.documentId,
    runSeq: running.runSeq,
    tier,
    versionId: version.id,
    characterCount: version.characterCount,
    lineageId: lineage.id,
    baseRoundId: lineage.base?.id ?? null,
  });
  const path = manuscriptPath(version.id);
  const document = { id: version.documentId, title: version.title, subtitle: version.subtitle, path };

  try {
    if (lineage.base === null) {
      await prism.writeAgentFiles(ctx.session.prismAgentId, [{ path, content: version.content }]);
      return confirmResult(round.id, ENUM_TO_TIER[tier], document);
    }

    const followup = await followupMaterials(round.id, lineage.id, lineage.base, ENUM_TO_TIER[tier]);
    await prism.writeAgentFiles(ctx.session.prismAgentId, [
      { path, content: version.content },
      ...(followup.previous.path === path ? [] : [{ path: followup.previous.path, content: lineage.base.content }]),
      ...followup.uploads,
    ]);

    return confirmResult(round.id, ENUM_TO_TIER[tier], document, { previous: followup.previous, seeds: seedsPrefix(round.id) });
  } catch (err) {
    await closeRound(round.id);
    throw err;
  }
};

export const reviewTools: Record<string, PrismToolHandler> = { 'confirm-review': confirmReview };

const onWorkflowLinked = async (tx: Transaction, workflow: PrismWorkflowRow): Promise<void> => {
  if (workflow.ref === null) return;

  const linked = await tx
    .update(PrismReviewRounds)
    .set({ workflowId: workflow.id })
    .where(
      and(
        eq(PrismReviewRounds.id, workflow.ref),
        eq(PrismReviewRounds.sessionId, workflow.sessionId),
        isNull(PrismReviewRounds.workflowId),
      ),
    )
    .returning({ id: PrismReviewRounds.id })
    .then(first);

  if (!linked)
    log.warn('review round already linked or not in this session: {ref} ({workflowId})', { ref: workflow.ref, workflowId: workflow.id });
};

const onWorkflowSettled = async (
  tx: Transaction,
  workflow: PrismWorkflowRow,
  outcome: WorkflowOutcome,
): Promise<(() => Promise<void>) | null> => {
  let result: ReviewOutcome | null = null;
  if (outcome.result !== null) {
    const parsed = ReviewOutcomeEnvelopeSchema.safeParse(outcome.result);
    if (!parsed.success) {
      log.error('review outcome rejected by envelope schema: {workflowId} {*}', {
        workflowId: workflow.prismWorkflowId,
        issues: parsed.error.issues,
      });
      Sentry.captureMessage(`prism review outcome malformed: ${workflow.prismWorkflowId}`, {
        level: 'error',
        extra: { workflowId: workflow.prismWorkflowId, issues: parsed.error.issues },
      });
      return null;
    }

    result = parsed.data as ReviewOutcome;
  }

  const round = await tx
    .update(PrismReviewRounds)
    .set({ result })
    .where(eq(PrismReviewRounds.workflowId, workflow.id))
    .returning({ id: PrismReviewRounds.id, documentId: PrismReviewRounds.documentId })
    .then(first);
  if (!round) return null;

  if (outcome.state === 'FAILED') {
    const refunded = await refundRound(tx, round.id);
    if (!refunded) return null;

    const failed = await tx
      .select({ userId: PrismSessions.userId })
      .from(PrismSessions)
      .where(eq(PrismSessions.id, workflow.sessionId))
      .then(firstOrThrow);

    return async () => {
      pubsub.publish('prism:credit', failed.userId, {});
    };
  }

  if (outcome.state !== 'COMPLETED') return null;

  const projected = await projectRoundThreads(tx, round.id);

  const document = await tx.select({ title: Documents.title }).from(Documents).where(eq(Documents.id, round.documentId)).then(first);
  const session = await tx
    .select({ userId: PrismSessions.userId })
    .from(PrismSessions)
    .where(eq(PrismSessions.id, workflow.sessionId))
    .then(firstOrThrow);

  // push는 트랜잭션 안에 남긴다 — 실패 시 settle 전체가 롤백돼 잡 재시도가 push까지 다시 태운다
  const { PUSH_TTL_SECONDS, sendPushNotificationOnce } = await import('#/external/firebase.ts');
  if (Date.now() - outcome.finishedAt.valueOf() <= PUSH_TTL_SECONDS * 1000) {
    const delivery = await sendPushNotificationOnce({
      key: `prism:push:review-done:${workflow.prismWorkflowId}`,
      userId: session.userId,
      title: `리뷰가 끝났어요 — 「${document?.title || '제목 없음'}」`,
      body: '결과가 정리돼 있어요.',
    });

    if (delivery === 'failed') throw new Error(`prism review push failed for workflow ${workflow.prismWorkflowId}`);
  }

  // 반환은 push 뒤에 — 크레딧 xact 락이 FCM 왕복 동안 유지되면 같은 유저의 채팅 정산·리뷰 확정이 lock_timeout에 걸린다
  const refunded = result?.kind === 'rejected' ? await refundRound(tx, round.id) : false;

  if (projected === null && !refunded) return null;

  // 완료 발행은 커밋 뒤여야 한다 — 이벤트를 받은 클라이언트가 즉시 재조회하는데, 커밋 전이면
  // result 없는 회차로 읽혀 목록에서 걸러지고, 다시 알려줄 이벤트가 없어 여백이 서지 못한다.
  return async () => {
    if (projected !== null) pubsub.publish('prism:review', projected.documentId, { roundId: round.id });
    if (refunded) pubsub.publish('prism:credit', session.userId, {});
  };
};

const onRunTerminal = async (executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void> => {
  await closePendingRounds(executor, sessionId, runSeq);
};

export const reviewHooks: PrismAppHooks = { onWorkflowLinked, onWorkflowSettled, onRunTerminal };
