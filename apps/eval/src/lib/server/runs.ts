import { and, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createChunks } from '../../../flows/src/text.ts';
import { createDb, Documents, PipelineRunDocs, PipelineRuns, PromptVariants, Variants } from './db/index.ts';
import type { RunDocStatus } from '../domain/admin-types.ts';

type Db = ReturnType<typeof createDb>;
type Env = App.Platform['env'];

const TERMINAL_DOC_STATUSES = new Set<RunDocStatus>(['done', 'failed', 'cancelled']);

export const spawnPipelineRun = async (
  db: Db,
  env: Env,
  input: { promptVariantId: string; corpusVersion: string },
): Promise<{ runId: string; spawnedCount: number; failedCount: number } | { error: string }> => {
  const [promptVariant] = await db
    .select({ label: PromptVariants.label })
    .from(PromptVariants)
    .where(eq(PromptVariants.id, input.promptVariantId))
    .limit(1);
  if (!promptVariant) {
    return { error: 'prompt variant not found' };
  }

  const docs = await db
    .select({ id: Documents.id, content: Documents.content })
    .from(Documents)
    .where(eq(Documents.corpusVersion, input.corpusVersion));
  if (docs.length === 0) {
    return { error: 'no documents for corpus version' };
  }

  const docsWithChunkCount = docs.map((doc) => ({ id: doc.id, chunkCount: createChunks(doc.content).length }));
  const totalChunks = docsWithChunkCount.reduce((sum, doc) => sum + doc.chunkCount, 0);

  await db
    .insert(Variants)
    .values({ id: nanoid(), label: promptVariant.label, round: input.corpusVersion, promptVariantId: input.promptVariantId })
    .onConflictDoUpdate({ target: Variants.label, set: { round: input.corpusVersion, promptVariantId: input.promptVariantId } });
  const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, promptVariant.label)).limit(1);

  const runId = nanoid();
  await db.insert(PipelineRuns).values({
    id: runId,
    kind: 'pipeline',
    variantId: variant.id,
    corpusVersion: input.corpusVersion,
    status: 'running',
    totalChunks,
    totalDocs: docsWithChunkCount.length,
  });

  let spawnedCount = 0;
  let failedCount = 0;

  for (const doc of docsWithChunkCount) {
    await db.insert(PipelineRunDocs).values({
      id: nanoid(),
      runId,
      documentId: doc.id,
      status: 'pending',
      totalChunks: doc.chunkCount,
    });

    try {
      const instance = await env.PIPELINE.create({
        params: {
          runId,
          promptVariantId: input.promptVariantId,
          variantLabel: promptVariant.label,
          corpusVersion: input.corpusVersion,
          documentId: doc.id,
        },
      });
      await db
        .update(PipelineRunDocs)
        .set({ workflowInstanceId: instance.id })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.id)));
      spawnedCount += 1;
    } catch (err) {
      const message = String(err).slice(0, 1000);
      console.warn(`pipeline spawn failed for document ${doc.id}: ${message}`);
      await db
        .update(PipelineRunDocs)
        .set({ status: 'failed', error: message })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.id)));
      failedCount += 1;
    }
  }

  return { runId, spawnedCount, failedCount };
};

export const spawnSamplingRun = async (
  db: Db,
  env: Env,
  input: { corpusVersion: string; size: number },
): Promise<{ runId: string; spawnedCount: number; failedCount: number }> => {
  const runId = nanoid();
  await db.insert(PipelineRuns).values({ id: runId, kind: 'sampling', corpusVersion: input.corpusVersion, status: 'running' });

  try {
    await env.SAMPLING.create({ id: runId, params: { runId, corpusVersion: input.corpusVersion, size: input.size } });
    return { runId, spawnedCount: 1, failedCount: 0 };
  } catch (err) {
    const message = String(err).slice(0, 1000);
    console.warn(`sampling spawn failed for run ${runId}: ${message}`);
    await db.update(PipelineRuns).set({ status: 'failed', error: message, finishedAt: new Date() }).where(eq(PipelineRuns.id, runId));
    return { runId, spawnedCount: 0, failedCount: 1 };
  }
};

const refreshPipelineDocs = async (db: Db, env: Env, runId: string): Promise<void> => {
  const docs = await db.select().from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, runId));

  for (const doc of docs) {
    if (TERMINAL_DOC_STATUSES.has(doc.status) || !doc.workflowInstanceId) continue;

    try {
      const instance = await env.PIPELINE.get(doc.workflowInstanceId);
      const status = await instance.status();
      if (status.status === 'errored') {
        await db
          .update(PipelineRunDocs)
          .set({ status: 'failed', error: (status.error?.message ?? 'workflow errored').slice(0, 1000) })
          .where(eq(PipelineRunDocs.id, doc.id));
      } else if (status.status === 'terminated') {
        await db.update(PipelineRunDocs).set({ status: 'cancelled' }).where(eq(PipelineRunDocs.id, doc.id));
      }
    } catch {
      // instance unreachable (e.g. flows worker not running locally) — leave status as-is
    }
  }

  const refreshed = await db.select({ status: PipelineRunDocs.status }).from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, runId));
  if (refreshed.length === 0 || refreshed.some((d) => !TERMINAL_DOC_STATUSES.has(d.status))) return;

  const doneCount = refreshed.filter((d) => d.status === 'done').length;
  const allDone = doneCount === refreshed.length;
  await db
    .update(PipelineRuns)
    .set({ status: allDone ? 'succeeded' : 'failed', doneDocs: doneCount, finishedAt: new Date() })
    .where(eq(PipelineRuns.id, runId));
};

const refreshSamplingInstance = async (db: Db, env: Env, runId: string): Promise<void> => {
  try {
    const instance = await env.SAMPLING.get(runId);
    const status = await instance.status();
    if (status.status === 'errored') {
      await db
        .update(PipelineRuns)
        .set({ status: 'failed', error: (status.error?.message ?? 'workflow errored').slice(0, 1000), finishedAt: new Date() })
        .where(eq(PipelineRuns.id, runId));
    } else if (status.status === 'terminated') {
      await db.update(PipelineRuns).set({ status: 'cancelled', finishedAt: new Date() }).where(eq(PipelineRuns.id, runId));
    }
  } catch {
    // instance unreachable (e.g. flows worker not running locally) — leave status as-is
  }
};

export const refreshRun = async (db: Db, env: Env, runId: string): Promise<void> => {
  const [run] = await db
    .select({ kind: PipelineRuns.kind, status: PipelineRuns.status })
    .from(PipelineRuns)
    .where(eq(PipelineRuns.id, runId))
    .limit(1);
  if (!run || run.status !== 'running') return;

  if (run.kind === 'pipeline') {
    await refreshPipelineDocs(db, env, runId);
  } else {
    await refreshSamplingInstance(db, env, runId);
  }
};

export const cancelRun = async (db: Db, env: Env, runId: string): Promise<{ ok: true } | { error: string }> => {
  const [run] = await db.select().from(PipelineRuns).where(eq(PipelineRuns.id, runId)).limit(1);
  if (!run) {
    return { error: 'run not found' };
  }

  if (run.kind === 'pipeline') {
    const docs = await db.select().from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, runId));
    for (const doc of docs) {
      if (TERMINAL_DOC_STATUSES.has(doc.status)) continue;

      if (doc.workflowInstanceId) {
        try {
          const instance = await env.PIPELINE.get(doc.workflowInstanceId);
          const status = await instance.status();
          if (status.status === 'running' || status.status === 'queued') {
            await instance.terminate();
          }
        } catch {
          // best-effort terminate; still mark cancelled below so partial results are preserved
        }
      }

      await db.update(PipelineRunDocs).set({ status: 'cancelled' }).where(eq(PipelineRunDocs.id, doc.id));
    }
  } else {
    try {
      const instance = await env.SAMPLING.get(runId);
      const status = await instance.status();
      if (status.status === 'running' || status.status === 'queued') {
        await instance.terminate();
      }
    } catch {
      // best-effort terminate
    }
  }

  await db.update(PipelineRuns).set({ status: 'cancelled', finishedAt: new Date() }).where(eq(PipelineRuns.id, runId));
  return { ok: true };
};

export const retryFailedDocs = async (db: Db, env: Env, runId: string): Promise<{ retried: number } | { error: string }> => {
  const [run] = await db.select().from(PipelineRuns).where(eq(PipelineRuns.id, runId)).limit(1);
  if (!run || run.kind !== 'pipeline' || !run.variantId) {
    return { error: 'pipeline run not found' };
  }

  const [variant] = await db.select().from(Variants).where(eq(Variants.id, run.variantId)).limit(1);
  if (!variant?.promptVariantId) {
    return { error: 'variant not resolved' };
  }

  const failedDocs = await db
    .select({ documentId: PipelineRunDocs.documentId })
    .from(PipelineRunDocs)
    .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.status, 'failed')));

  let retriedCount = 0;

  for (const doc of failedDocs) {
    await db
      .update(PipelineRunDocs)
      .set({ status: 'pending', error: null, workflowInstanceId: null })
      .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.documentId)));

    try {
      await env.PIPELINE.create({
        params: {
          runId,
          promptVariantId: variant.promptVariantId,
          variantLabel: variant.label,
          corpusVersion: run.corpusVersion,
          documentId: doc.documentId,
        },
      });
      retriedCount += 1;
    } catch (err) {
      const message = String(err).slice(0, 1000);
      console.warn(`retry spawn failed for document ${doc.documentId}: ${message}`);
      await db
        .update(PipelineRunDocs)
        .set({ status: 'failed', error: message })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.documentId)));
    }
  }

  if (retriedCount > 0) {
    await db.update(PipelineRuns).set({ status: 'running', finishedAt: null }).where(eq(PipelineRuns.id, runId));
  }

  return { retried: retriedCount };
};
