import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import { quoteReviewCredits, ReviewOutcomeEnvelopeSchema } from '@typie/prism';
import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
import { PgTransaction } from 'drizzle-orm/pg-core';
import {
  db,
  Documents,
  first,
  firstOrThrow,
  PrismReviewDocumentVersions,
  PrismReviewRounds,
  PrismSessions,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { activeRun, prism } from '#/external/prism.ts';
import { pubsub } from '#/pubsub.ts';
import { assertDocumentPermission } from './permission.ts';
import { chargePrismCredit, lockUserPrismCredit, readPrismCreditBalance, refundPrismReview } from './prism-credit.ts';
import { toMilli } from './prism-credit-core.ts';
import { snapshotManuscript } from './prism-manuscript.ts';
import { ConfirmInputSchema, confirmResult, ENUM_TO_TIER, manuscriptPath, pickVersion } from './prism-review-core.ts';
import { projectRoundThreads } from './prism-review-threads.ts';
import type { PrismReviewTier } from '@typie/lib/enums';
import type { ReviewOutcome } from '@typie/prism';
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

const confirmReview = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ConfirmInputSchema.safeParse(input);
  if (!parsed.success) throw new TypieError({ code: 'invalid_confirm_input', status: 400 });
  if (parsed.data.decision === 'declined') return { decision: 'declined' } as const;

  const { versionId, tier } = parsed.data;
  if (!validateDbId(TableCode.PRISM_REVIEW_DOCUMENT_VERSIONS).regex.test(versionId))
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

  const round = await createRound({
    userId: ctx.userId,
    sessionId: ctx.session.id,
    documentId: version.documentId,
    runSeq: running.runSeq,
    tier,
    versionId: version.id,
    characterCount: version.characterCount,
  });
  const path = manuscriptPath(version.id);

  try {
    await prism.writeAgentFiles(ctx.session.prismAgentId, [{ path, content: version.content }]);
  } catch (err) {
    await closeRound(round.id);
    throw err;
  }

  return confirmResult(round.id, ENUM_TO_TIER[tier], { id: version.documentId, title: version.title, subtitle: version.subtitle, path });
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
