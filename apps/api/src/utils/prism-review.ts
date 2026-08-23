import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, desc, eq, isNull } from 'drizzle-orm';
import {
  db,
  dbr,
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
import { readMergedGraph } from './changeset.ts';
import { assertDocumentPermission } from './permission.ts';
import { ConfirmInputSchema, confirmResult, ENUM_TO_TIER, manuscriptPath, pickVersion } from './prism-review-core.ts';
import { projectRoundThreads } from './prism-review-threads.ts';
import { wasmThread } from './wasm-thread.ts';
import type { PrismReviewTier } from '@typie/lib/enums';
import type { ReviewOutcome, WorkflowState } from '@typie/prism';
import type { Database, Transaction } from '#/db/index.ts';
import type { PrismAppHooks, PrismWorkflowRow } from './prism-apps.ts';
import type { Snapshot } from './prism-review-core.ts';
import type { PrismToolContext, PrismToolHandler } from './prism-tools.ts';

const log = logger.getChild('prism-review');

type Manuscript = Snapshot & { characterCount: number };

const snapshotManuscript = async (documentId: string): Promise<Manuscript> => {
  const head = await dbr
    .select({ title: Documents.title, subtitle: Documents.subtitle })
    .from(Documents)
    .where(eq(Documents.id, documentId))
    .then(first);
  if (!head) throw new TypieError({ code: 'not_found', status: 404 });

  const graph = await readMergedGraph(documentId);
  if (graph.length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  const extracted = await wasmThread.extractProse(graph).catch(() => {
    throw new TypieError({ code: 'prism_extract_failed', status: 502 });
  });

  const content = extracted.result.text;
  if (content === null || content.trim().length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  return { title: head.title, subtitle: head.subtitle, content, characterCount: extracted.result.characterCount };
};

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

const onWorkflowSettled = async (workflow: PrismWorkflowRow, view: WorkflowState): Promise<void> => {
  const round = await db
    .update(PrismReviewRounds)
    .set({ result: view.workflow.result === null ? null : (JSON.parse(view.workflow.result) as ReviewOutcome) })
    .where(eq(PrismReviewRounds.workflowId, workflow.id))
    .returning({ id: PrismReviewRounds.id, documentId: PrismReviewRounds.documentId })
    .then(first);
  if (!round || view.workflow.status !== 'completed') return;

  await projectRoundThreads(round.id);

  const document = await db.select({ title: Documents.title }).from(Documents).where(eq(Documents.id, round.documentId)).then(first);
  const session = await db
    .select({ userId: PrismSessions.userId })
    .from(PrismSessions)
    .where(eq(PrismSessions.id, workflow.sessionId))
    .then(firstOrThrow);

  const { sendPushNotificationOnce } = await import('#/external/firebase.ts');
  const delivery = await sendPushNotificationOnce({
    key: `prism:push:review-done:${workflow.prismWorkflowId}`,
    userId: session.userId,
    title: `리뷰가 끝났어요 — 「${document?.title || '제목 없음'}」`,
    body: '결과가 정리돼 있어요.',
  });

  if (delivery === 'failed') throw new Error(`prism review push failed for workflow ${workflow.prismWorkflowId}`);
};

const onRunTerminal = async (sessionId: string, runSeq: number): Promise<void> => {
  await closePendingRounds(db, sessionId, runSeq);
};

const resolveSession = async (ref: string | null): Promise<string | null> => {
  if (ref === null) return null;
  const round = await db
    .select({ sessionId: PrismReviewRounds.sessionId })
    .from(PrismReviewRounds)
    .where(eq(PrismReviewRounds.id, ref))
    .then(first);
  return round?.sessionId ?? null;
};

export const reviewHooks: PrismAppHooks = { onWorkflowLinked, onWorkflowSettled, onRunTerminal, resolveSession };
