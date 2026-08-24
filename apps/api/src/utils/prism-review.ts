import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import { ReviewOutcomeEnvelopeSchema } from '@typie/prism';
import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
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

const upsertDocumentVersion = async (tx: Transaction, documentId: string, snap: Manuscript): Promise<{ id: string } | null> => {
  const latest = await tx
    .select({
      id: PrismReviewDocumentVersions.id,
      version: PrismReviewDocumentVersions.version,
      content: PrismReviewDocumentVersions.content,
      title: PrismReviewDocumentVersions.title,
      subtitle: PrismReviewDocumentVersions.subtitle,
    })
    .from(PrismReviewDocumentVersions)
    .where(eq(PrismReviewDocumentVersions.documentId, documentId))
    .orderBy(desc(PrismReviewDocumentVersions.version))
    .limit(1)
    .then(first);

  const picked = pickVersion(latest ?? null, snap);
  if (latest && picked.reuse) return { id: latest.id };

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
    .returning({ id: PrismReviewDocumentVersions.id })
    .then(first);

  return inserted ?? null;
};

const closePendingRounds = async (executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void> => {
  await executor
    .update(PrismReviewRounds)
    .set({ closedAt: dayjs() })
    .where(
      and(
        eq(PrismReviewRounds.sessionId, sessionId),
        eq(PrismReviewRounds.prismRunSeq, runSeq),
        isNull(PrismReviewRounds.workflowId),
        isNull(PrismReviewRounds.closedAt),
      ),
    );
};

const createRound = async (input: {
  sessionId: string;
  documentId: string;
  runSeq: number;
  tier: PrismReviewTier;
  snap: Manuscript;
}): Promise<{ id: string; versionId: string }> => {
  for (let attempt = 0; attempt < 2; attempt++) {
    const created = await db.transaction(async (tx) => {
      const version = await upsertDocumentVersion(tx, input.documentId, input.snap);
      if (!version) return null;

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
          documentVersionId: version.id,
        })
        .onConflictDoNothing({ target: [PrismReviewRounds.documentId, PrismReviewRounds.round] })
        .returning({ id: PrismReviewRounds.id })
        .then(first);

      return row ? { id: row.id, versionId: version.id } : null;
    });

    if (created) return created;
  }

  throw new TypieError({ code: 'prism_round_conflict', status: 409 });
};

const closeRound = async (roundId: string): Promise<void> => {
  await db
    .update(PrismReviewRounds)
    .set({ closedAt: dayjs() })
    .where(and(eq(PrismReviewRounds.id, roundId), isNull(PrismReviewRounds.closedAt)));
};

const confirmReview = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ConfirmInputSchema.safeParse(input);
  if (!parsed.success) throw new TypieError({ code: 'invalid_confirm_input', status: 400 });
  if (parsed.data.decision === 'declined') return { decision: 'declined' } as const;

  const { documentId, tier } = parsed.data;
  if (!validateDbId(TableCode.DOCUMENTS).regex.test(documentId)) throw new TypieError({ code: 'invalid_confirm_input', status: 400 });

  await assertDocumentPermission({ userId: ctx.userId, documentId });

  const running = activeRun(ctx.agent.runs);
  if (!running) throw new TypieError({ code: 'prism_tool_settled', status: 409 });

  const snap = await snapshotManuscript(documentId);
  const round = await createRound({ sessionId: ctx.session.id, documentId, runSeq: running.runSeq, tier, snap });
  const path = manuscriptPath(round.versionId);

  try {
    await prism.writeAgentFiles(ctx.session.prismAgentId, [{ path, content: snap.content }]);
  } catch (err) {
    await closeRound(round.id);
    throw err;
  }

  return confirmResult(round.id, ENUM_TO_TIER[tier], { id: documentId, title: snap.title, subtitle: snap.subtitle, path });
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
  if (!round || outcome.state !== 'COMPLETED') return null;

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

  if (projected === null) return null;

  // 완료 발행은 커밋 뒤여야 한다 — 이벤트를 받은 클라이언트가 즉시 재조회하는데, 커밋 전이면
  // result 없는 회차로 읽혀 목록에서 걸러지고, 다시 알려줄 이벤트가 없어 여백이 서지 못한다.
  return async () => {
    pubsub.publish('prism:review', projected.documentId, { roundId: round.id });
  };
};

const onRunTerminal = async (executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void> => {
  await closePendingRounds(executor, sessionId, runSeq);
};

export const reviewHooks: PrismAppHooks = { onWorkflowLinked, onWorkflowSettled, onRunTerminal };
