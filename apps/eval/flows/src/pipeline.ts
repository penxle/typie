import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, eq, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import {
  createDb,
  Documents,
  Feedbacks,
  FeedbackSets,
  metaCacheKey,
  PipelineRunDocs,
  PipelineRuns,
  PromptVariants,
  readStageCache,
  summarizeCacheKey,
  Variants,
  writeStageCache,
} from './db.ts';
import { analyzeChunkWithContext, analyzeGlobal, createOpenAI, resolveStagePrompt, runTool } from './llm.ts';
import { buildFeedbackTool, buildMetaTool, buildSummaryTool, createChunks, createFindRange } from './text.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { Db } from './db.ts';
import type { FlowEnv, PipelineParams } from './index.ts';
import type { Usage } from './llm.ts';
import type { MetaStructured, SummaryStructured } from './text.ts';

const SUMMARIZE_BATCH = 8;
const ANALYZE_BATCH = 4;
const LLM_STEP = { retries: { limit: 2, delay: '10 seconds' as const, backoff: 'exponential' as const }, timeout: '5 minutes' as const };

type FeedbackRow = {
  startText: string;
  endText: string;
  matchStart: number | null;
  matchEnd: number | null;
  category: string | null;
  body: string;
};

const emptyUsage = (): Usage => ({ promptTokens: 0, completionTokens: 0 });

const finiteTokens = (n: number): number => (Number.isFinite(n) ? Math.round(n) : 0);

const addRunUsage = async (db: Db, runId: string, usage: Usage): Promise<void> => {
  const prompt = finiteTokens(usage.promptTokens);
  const completion = finiteTokens(usage.completionTokens);
  if (prompt === 0 && completion === 0) return;
  await db
    .update(PipelineRuns)
    .set({
      promptTokens: sql`${PipelineRuns.promptTokens} + ${prompt}`,
      completionTokens: sql`${PipelineRuns.completionTokens} + ${completion}`,
    })
    .where(eq(PipelineRuns.id, runId));
};

export class PipelineWorkflow extends WorkflowEntrypoint<FlowEnv, PipelineParams> {
  async run(event: WorkflowEvent<PipelineParams>, step: WorkflowStep) {
    const { runId, promptVariantId, variantLabel, corpusVersion, documentId } = event.payload;
    const instanceId = event.instanceId;
    const db = createDb(this.env.DB);
    const openai = createOpenAI(this.env.CLOUDFLARE_API_KEY, this.env.CLOUDFLARE_AIGATEWAY_URL);

    const resolved = await step.do('resolve', async () => {
      const [pv] = await db
        .select({ content: PromptVariants.content })
        .from(PromptVariants)
        .where(eq(PromptVariants.id, promptVariantId))
        .limit(1);
      if (!pv) throw new Error(`prompt variant not found: ${promptVariantId}`);

      const [doc] = await db.select({ content: Documents.content }).from(Documents).where(eq(Documents.id, documentId)).limit(1);
      if (!doc) throw new Error(`document not found: ${documentId}`);

      const summarize = await resolveStagePrompt(pv.content.summarize);
      const meta = await resolveStagePrompt(pv.content.meta);
      const analyze = await resolveStagePrompt(pv.content.analyze);
      const chunks = createChunks(doc.content);

      await db
        .insert(Variants)
        .values({ id: nanoid(), label: variantLabel, round: corpusVersion, promptVariantId })
        .onConflictDoUpdate({ target: Variants.label, set: { promptVariantId } });
      const [variant] = await db.select({ id: Variants.id }).from(Variants).where(eq(Variants.label, variantLabel)).limit(1);

      await db.update(PromptVariants).set({ status: 'ran' }).where(eq(PromptVariants.id, promptVariantId));

      const [existingDoc] = await db
        .select()
        .from(PipelineRunDocs)
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)))
        .limit(1);

      if (existingDoc?.status === 'done') {
        return { alreadyDone: true as const };
      }

      if (existingDoc) {
        const isNewInstance = existingDoc.workflowInstanceId !== instanceId;
        const prevDone = existingDoc.doneChunks;
        await db
          .update(PipelineRunDocs)
          .set({ workflowInstanceId: instanceId, status: 'running', totalChunks: chunks.length, doneChunks: 0, error: null })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
        if (isNewInstance && prevDone > 0) {
          await db
            .update(PipelineRuns)
            .set({ doneChunks: sql`${PipelineRuns.doneChunks} - ${prevDone}` })
            .where(eq(PipelineRuns.id, runId));
        }
      } else {
        await db.insert(PipelineRunDocs).values({
          id: nanoid(),
          runId,
          documentId,
          workflowInstanceId: instanceId,
          status: 'running',
          totalChunks: chunks.length,
          doneChunks: 0,
        });
      }

      return {
        alreadyDone: false as const,
        variantId: variant.id,
        content: doc.content,
        chunks,
        summarize,
        meta,
        analyze,
      };
    });

    if (resolved.alreadyDone) {
      return { skipped: true };
    }

    const { variantId, content, chunks, summarize, meta: metaPrompt, analyze } = resolved;

    try {
      const summaries: SummaryStructured[] = [];
      for (let b = 0; b < chunks.length; b += SUMMARIZE_BATCH) {
        const slice = chunks.slice(b, b + SUMMARIZE_BATCH).map((chunk, k) => ({ index: b + k, text: chunk.text }));
        const batch = await step.do(`summarize-${b}`, LLM_STEP, async () => {
          const usage = emptyUsage();
          const out = await Promise.all(
            slice.map(async ({ index, text }) => {
              const key = summarizeCacheKey(summarize.hash, documentId, index);
              const cached = await readStageCache<SummaryStructured>(db, key);
              if (cached) return cached;
              const s = await runTool<SummaryStructured>(summarize, buildSummaryTool(summarize.toolDescriptions), text, openai, usage);
              await writeStageCache(db, key, s);
              return s;
            }),
          );
          return { summaries: out, usage };
        });
        summaries.push(...batch.summaries);
        await step.do(`summarize-commit-${b}`, () => addRunUsage(db, runId, batch.usage));
      }

      const meta = await step.do('meta', LLM_STEP, async () => {
        const key = metaCacheKey(summarize.hash, metaPrompt.hash, documentId);
        const cached = await readStageCache<MetaStructured>(db, key);
        if (cached) return { meta: cached, usage: emptyUsage() };
        const usage = emptyUsage();
        const resolvedMeta = await analyzeGlobal(metaPrompt, buildMetaTool(metaPrompt.toolDescriptions), summaries, openai, usage);
        await writeStageCache(db, key, resolvedMeta);
        return { meta: resolvedMeta, usage };
      });
      await step.do('meta-commit', () => addRunUsage(db, runId, meta.usage));

      const findRange = createFindRange(content);
      const allFeedbacks: FeedbackRow[] = [];

      for (let b = 0; b < chunks.length; b += ANALYZE_BATCH) {
        const slice = chunks.slice(b, b + ANALYZE_BATCH).map((chunk, k) => ({ index: b + k, text: chunk.text, start: chunk.start }));
        const batch = await step.do(`analyze-${b}`, LLM_STEP, async () => {
          const usage = emptyUsage();
          const rows = await Promise.all(
            slice.map(async ({ index, text, start }) => {
              const collected: FeedbackRow[] = [];
              await analyzeChunkWithContext(
                analyze,
                buildFeedbackTool(analyze.toolDescriptions),
                {
                  meta: meta.meta,
                  precedingNarrative: index > 0 ? (summaries[index - 1]?.narrative ?? '') : '',
                  followingNarrative: index < chunks.length - 1 ? (summaries[index + 1]?.narrative ?? '') : '',
                  currentText: text,
                },
                (feedback) => {
                  const range = findRange(feedback.start, feedback.end, start);
                  collected.push({
                    startText: feedback.start,
                    endText: feedback.end,
                    matchStart: range?.rangeStart ?? null,
                    matchEnd: range?.rangeEnd ?? null,
                    category: feedback.category ?? null,
                    body: feedback.feedback,
                  });
                },
                openai,
                usage,
              );
              return collected;
            }),
          );
          return { feedbacks: rows.flat(), usage, chunkCount: slice.length };
        });
        allFeedbacks.push(...batch.feedbacks);
        await step.do(`analyze-commit-${b}`, async () => {
          await db
            .update(PipelineRunDocs)
            .set({ doneChunks: sql`${PipelineRunDocs.doneChunks} + ${batch.chunkCount}` })
            .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
          await db
            .update(PipelineRuns)
            .set({ doneChunks: sql`${PipelineRuns.doneChunks} + ${batch.chunkCount}` })
            .where(eq(PipelineRuns.id, runId));
          await addRunUsage(db, runId, batch.usage);
        });
      }

      await step.do('complete', async () => {
        const existingSets = await db
          .select({ id: FeedbackSets.id })
          .from(FeedbackSets)
          .where(and(eq(FeedbackSets.runId, runId), eq(FeedbackSets.documentId, documentId)));
        for (const set of existingSets) {
          await db.delete(Feedbacks).where(eq(Feedbacks.setId, set.id));
        }
        await db.delete(FeedbackSets).where(and(eq(FeedbackSets.runId, runId), eq(FeedbackSets.documentId, documentId)));

        const setId = nanoid();
        await db.insert(FeedbackSets).values({ id: setId, runId, documentId, variantId });
        for (const [ord, feedback] of allFeedbacks.entries()) {
          await db.insert(Feedbacks).values({ id: nanoid(), setId, ord, ...feedback });
        }
        await db
          .update(PipelineRunDocs)
          .set({ status: 'done', doneChunks: chunks.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { done: true, feedbacks: allFeedbacks.length };
    } catch (err) {
      const message = String(err).slice(0, 1000);
      await step.do('mark-failed', async () => {
        await db
          .update(PipelineRunDocs)
          .set({ status: 'failed', error: message })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });
      return { failed: true };
    }
  }
}
