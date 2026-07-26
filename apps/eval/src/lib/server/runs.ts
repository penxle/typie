import { and, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createChunks } from '../../../flows/src/text.ts';
import { AnalysisPromptSets, createDb, Documents, PipelineRunDocs, PipelineRuns, PromptVariants, Variants } from './db/index.ts';
import type { RunDocStatus } from '../domain/admin-types.ts';

type Db = ReturnType<typeof createDb>;
type Env = App.Platform['env'];

const TERMINAL_DOC_STATUSES = new Set<RunDocStatus>(['done', 'failed', 'cancelled']);

export const spawnPipelineRun = async (
  db: Db,
  env: Env,
  input: { promptVariantId: string; corpusVersion: string; documentIds?: string[] },
): Promise<{ runId: string; spawnedCount: number; failedCount: number } | { error: string }> => {
  const [promptVariant] = await db
    .select({ label: PromptVariants.label, content: PromptVariants.content })
    .from(PromptVariants)
    .where(eq(PromptVariants.id, input.promptVariantId))
    .limit(1);
  if (!promptVariant) {
    return { error: 'prompt variant not found' };
  }
  // 구 파이프라인은 단계마다 모델이 달라 대개 둘 이상이 담긴다 — 그때는 금액을 내지 않는다.
  const models = [...new Set(Object.values(promptVariant.content).map((p) => p.model))];

  const docs = await db
    .select({ id: Documents.id, content: Documents.content })
    .from(Documents)
    .where(
      input.documentIds
        ? and(eq(Documents.corpusVersion, input.corpusVersion), inArray(Documents.id, input.documentIds))
        : eq(Documents.corpusVersion, input.corpusVersion),
    );
  if (docs.length === 0) {
    return { error: 'no documents for corpus version' };
  }
  // 지정한 문서가 이 코퍼스에 없으면 조용히 적게 도는 대신 실패시킨다 — 부분집합 실행의 결과를
  // 전체 실행과 헷갈리게 두지 않는다.
  if (input.documentIds && docs.length !== input.documentIds.length) {
    return { error: `documents not found in corpus: ${input.documentIds.filter((id) => docs.every((d) => d.id !== id)).join(', ')}` };
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
    meta: { models },
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

// 재설계 파이프라인 실행. 문서마다 워크플로 인스턴스를 띄우므로 문서 단위 병렬은 여기서,
// 창·묶음 단위 병렬은 워크플로 안에서 이뤄진다.
export const spawnAnalysisRun = async (
  db: Db,
  env: Env,
  input: { promptSetId: string; corpusVersion: string; documentIds?: string[] },
): Promise<{ runId: string; spawnedCount: number; failedCount: number } | { error: string }> => {
  const [promptSet] = await db
    .select({ label: AnalysisPromptSets.label, content: AnalysisPromptSets.content })
    .from(AnalysisPromptSets)
    .where(eq(AnalysisPromptSets.id, input.promptSetId))
    .limit(1);
  if (!promptSet) {
    return { error: 'prompt set not found' };
  }
  // 비용 환산용. 프롬프트는 나중에 고쳐질 수 있어 실행 시각의 모델을 그대로 박아둔다.
  const models = [...new Set(Object.values(promptSet.content).map((p) => p.model))];

  const docs = await db
    .select({ id: Documents.id })
    .from(Documents)
    .where(
      input.documentIds
        ? and(eq(Documents.corpusVersion, input.corpusVersion), inArray(Documents.id, input.documentIds))
        : eq(Documents.corpusVersion, input.corpusVersion),
    );
  if (docs.length === 0) {
    return { error: 'no documents for corpus version' };
  }
  if (input.documentIds && docs.length !== input.documentIds.length) {
    return { error: `documents not found in corpus: ${input.documentIds.filter((id) => docs.every((d) => d.id !== id)).join(', ')}` };
  }

  await db
    .insert(Variants)
    .values({ id: nanoid(), label: promptSet.label, round: input.corpusVersion, promptVariantId: null })
    .onConflictDoUpdate({ target: Variants.label, set: { round: input.corpusVersion } });
  const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, promptSet.label)).limit(1);

  const runId = nanoid();
  await db.insert(PipelineRuns).values({
    id: runId,
    kind: 'analysis',
    variantId: variant.id,
    corpusVersion: input.corpusVersion,
    status: 'running',
    totalDocs: docs.length,
    // 재실행하려면 어느 프롬프트 세트로 돌았는지 알아야 한다. 라벨로 되짚을 수도 있지만
    // 그 사이 세트 라벨이 바뀌면 엉뚱한 세트로 다시 돌게 된다.
    meta: { promptSetId: input.promptSetId, models },
  });

  let spawnedCount = 0;
  let failedCount = 0;

  for (const doc of docs) {
    await db.insert(PipelineRunDocs).values({ id: nanoid(), runId, documentId: doc.id, status: 'pending', phase: 'queued' });
    try {
      const instance = await env.ANALYSIS.create({
        params: {
          runId,
          promptSetId: input.promptSetId,
          variantLabel: variant.id,
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
      console.warn(`analysis spawn failed for document ${doc.id}: ${message}`);
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

// 구 파이프라인과 재설계 파이프라인은 문서 단위 워크플로라는 점이 같고 바인딩만 다르다.
const refreshDocs = async (db: Db, workflow: Env['PIPELINE'] | Env['ANALYSIS'], runId: string): Promise<void> => {
  const docs = await db.select().from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, runId));

  for (const doc of docs) {
    if (TERMINAL_DOC_STATUSES.has(doc.status) || !doc.workflowInstanceId) continue;

    try {
      const instance = await workflow.get(doc.workflowInstanceId);
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
    await refreshDocs(db, env.PIPELINE, runId);
  } else if (run.kind === 'analysis') {
    await refreshDocs(db, env.ANALYSIS, runId);
  } else {
    await refreshSamplingInstance(db, env, runId);
  }
};

export const cancelRun = async (db: Db, env: Env, runId: string): Promise<{ ok: true } | { error: string }> => {
  const [run] = await db.select().from(PipelineRuns).where(eq(PipelineRuns.id, runId)).limit(1);
  if (!run) {
    return { error: 'run not found' };
  }

  if (run.kind === 'pipeline' || run.kind === 'analysis') {
    const workflow = run.kind === 'pipeline' ? env.PIPELINE : env.ANALYSIS;
    const docs = await db.select().from(PipelineRunDocs).where(eq(PipelineRunDocs.runId, runId));
    for (const doc of docs) {
      if (TERMINAL_DOC_STATUSES.has(doc.status)) continue;

      if (doc.workflowInstanceId) {
        try {
          const instance = await workflow.get(doc.workflowInstanceId);
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
  if (!run || (run.kind !== 'pipeline' && run.kind !== 'analysis') || !run.variantId) {
    return { error: 'run not found' };
  }

  const [variant] = await db.select().from(Variants).where(eq(Variants.id, run.variantId)).limit(1);
  if (!variant) {
    return { error: 'variant not resolved' };
  }

  // 어느 워크플로를 어떤 인자로 띄울지는 여기서 한 번 정한다 — 반복문 안에서 갈래를 다시 세면
  // 좁혀둔 타입이 풀리고, 무엇보다 실행 종류마다 다른 인자를 한자리에서 볼 수 있다.
  let spawnDoc: (documentId: string) => Promise<{ id: string }>;

  if (run.kind === 'analysis') {
    // 분석 실행은 프롬프트 세트가 있어야 다시 돈다. 옛 실행에는 meta가 없어 라벨로 되짚는다.
    let promptSetId = ((run.meta as { promptSetId?: unknown } | null)?.promptSetId as string | undefined) ?? null;
    if (!promptSetId) {
      const [set] = await db
        .select({ id: AnalysisPromptSets.id })
        .from(AnalysisPromptSets)
        .where(eq(AnalysisPromptSets.label, variant.label))
        .limit(1);
      promptSetId = set?.id ?? null;
    }
    if (!promptSetId) {
      return { error: 'prompt set not resolved' };
    }
    const resolvedSetId = promptSetId;
    spawnDoc = (documentId) =>
      env.ANALYSIS.create({
        params: { runId, promptSetId: resolvedSetId, variantLabel: variant.id, corpusVersion: run.corpusVersion, documentId },
      });
  } else {
    const promptVariantId = variant.promptVariantId;
    if (!promptVariantId) {
      return { error: 'variant not resolved' };
    }
    spawnDoc = (documentId) =>
      env.PIPELINE.create({
        params: { runId, promptVariantId, variantLabel: variant.label, corpusVersion: run.corpusVersion, documentId },
      });
  }

  // 취소된 문서도 재시도 대상 — 취소된 run을 새 run 없이 재개해 완료 문서의 재지불을 피한다.
  // done 문서는 여기서 애초에 스폰되지 않는 것이 스킵의 본체다.
  const retryableDocs = await db
    .select({ documentId: PipelineRunDocs.documentId })
    .from(PipelineRunDocs)
    .where(and(eq(PipelineRunDocs.runId, runId), inArray(PipelineRunDocs.status, ['failed', 'cancelled'])));

  let retriedCount = 0;

  for (const doc of retryableDocs) {
    await db
      .update(PipelineRunDocs)
      .set({ status: 'pending', error: null, workflowInstanceId: null, ...(run.kind === 'analysis' && { phase: 'queued' }) })
      .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.documentId)));

    try {
      const instance = await spawnDoc(doc.documentId);
      await db
        .update(PipelineRunDocs)
        .set({ workflowInstanceId: instance.id })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, doc.documentId)));
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
