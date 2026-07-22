import { WorkflowEntrypoint } from 'cloudflare:workers';
import { eq, sql } from 'drizzle-orm';
import { createDb, Documents, PipelineRuns, readStageCache, StageCache } from './db.ts';
import { createInternalApi } from './internal-api.ts';
import { classifyLiterary, createOpenAI } from './llm.ts';
import { corpusConflict, fillQuotas, pickLiteraryDocs, selectSuccessfulExtracts, stratifySelection } from './sampling-select.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { RunPhase } from '../../src/lib/domain/admin-types.ts';
import type { Db } from './db.ts';
import type { FlowEnv, SamplingParams } from './index.ts';
import type { Classified, LiteraryDoc, SelectedDocument } from './sampling-select.ts';

const CLASSIFY_MODEL = 'google-vertex-ai/google/gemini-3.5-flash-lite';
const CLASSIFY_BATCH = 8;
const EXTRACT_BATCH = 5;
const CANDIDATE_INSERT_BATCH = 20;
const LLM_STEP = { retries: { limit: 2, delay: '10 seconds' as const, backoff: 'exponential' as const }, timeout: '5 minutes' as const };

const candidateKey = (runId: string, documentId: string): string => `sample/${runId}/candidate/${documentId}`;

const setPhase = async (db: Db, runId: string, phase: RunPhase): Promise<void> => {
  await db.update(PipelineRuns).set({ phase }).where(eq(PipelineRuns.id, runId));
};

const addDoneDocs = async (db: Db, runId: string, n: number): Promise<void> => {
  await db
    .update(PipelineRuns)
    .set({ doneDocs: sql`${PipelineRuns.doneDocs} + ${n}` })
    .where(eq(PipelineRuns.id, runId));
};

export class SamplingWorkflow extends WorkflowEntrypoint<FlowEnv, SamplingParams> {
  async run(event: WorkflowEvent<SamplingParams>, step: WorkflowStep) {
    const { runId, corpusVersion, size } = event.payload;
    const db = createDb(this.env.DB);
    const api = createInternalApi(this.env.INTERNAL_API_BASE, this.env.INTERNAL_API_KEY);
    const openai = createOpenAI(this.env.CLOUDFLARE_API_KEY, this.env.CLOUDFLARE_AIGATEWAY_URL);

    try {
      const candidateIds = await step.do('candidates', async () => {
        await setPhase(db, runId, 'candidates');
        const candidates = await api.candidates({ limit: 400 });
        for (let i = 0; i < candidates.length; i += CANDIDATE_INSERT_BATCH) {
          await db
            .insert(StageCache)
            .values(
              candidates
                .slice(i, i + CANDIDATE_INSERT_BATCH)
                .map((c) => ({ key: candidateKey(runId, c.documentId), value: { text: c.text } })),
            )
            .onConflictDoNothing();
        }
        await db.update(PipelineRuns).set({ totalDocs: candidates.length, doneDocs: 0 }).where(eq(PipelineRuns.id, runId));
        return candidates.map((c) => c.documentId);
      });

      const literaryDocs: LiteraryDoc[] = [];
      await step.do('phase-classify', () => setPhase(db, runId, 'classify'));
      for (let b = 0; b < candidateIds.length; b += CLASSIFY_BATCH) {
        const batchIds = candidateIds.slice(b, b + CLASSIFY_BATCH);
        const found = await step.do(`classify-${b}`, LLM_STEP, async () => {
          const classified = await Promise.all(
            batchIds.map(async (documentId): Promise<Classified> => {
              const cached = await readStageCache<{ text: string }>(db, candidateKey(runId, documentId));
              const text = cached?.text ?? '';
              try {
                const result = await classifyLiterary(openai, CLASSIFY_MODEL, text);
                return {
                  candidate: { documentId, text: '', characterCount: 0 },
                  literary: result.literary,
                  kind: result.kind,
                  genre: result.genre,
                };
              } catch {
                return { candidate: { documentId, text: '', characterCount: 0 }, literary: false, kind: 'error', genre: 'etc' };
              }
            }),
          );
          return pickLiteraryDocs(classified);
        });
        literaryDocs.push(...found);
        await step.do(`classify-progress-${b}`, () => addDoneDocs(db, runId, batchIds.length));
      }

      const { genreDist, allocation, picks } = await step.do('select-strata', () => Promise.resolve(stratifySelection(literaryDocs, size)));

      await step.do('phase-extract', async () => {
        await setPhase(db, runId, 'extract');
        await db.update(PipelineRuns).set({ totalDocs: picks.length, doneDocs: 0 }).where(eq(PipelineRuns.id, runId));
      });

      const genreByRef = new Map(picks.map((p) => [p.documentId, p.genre]));
      const extracted: SelectedDocument[] = [];
      let batchNo = 0;
      for (let cursor = 0; cursor < picks.length; cursor += EXTRACT_BATCH) {
        const batchIds = picks.slice(cursor, cursor + EXTRACT_BATCH).map((p) => p.documentId);
        batchNo += 1;
        const good = await step.do(`extract-${batchNo}`, LLM_STEP, async () => {
          const results = await api.extract(batchIds);
          return selectSuccessfulExtracts(results, () => crypto.randomUUID());
        });
        extracted.push(...good);
        await step.do(`extract-progress-${batchNo}`, () => addDoneDocs(db, runId, batchIds.length));
      }

      const selected = fillQuotas(
        extracted.map((d) => ({ ...d, genre: genreByRef.get(d.refId) ?? 'etc' })),
        allocation,
        size,
      );

      if (selected.length < size) {
        throw new Error(`insufficient documents after extraction: ${selected.length}/${size}`);
      }

      await step.do('corpus-guard', async () => {
        const existing = await db.select({ refId: Documents.refId }).from(Documents).where(eq(Documents.corpusVersion, corpusVersion));
        if (
          corpusConflict(
            existing.map((e) => e.refId),
            selected.map((d) => d.refId),
          )
        ) {
          throw new Error(`corpus version already frozen: ${corpusVersion}`);
        }
      });

      await step.do('freeze', async () => {
        await setPhase(db, runId, 'freeze');
        await db
          .insert(Documents)
          .values(
            selected.map((d) => ({
              id: d.id,
              refId: d.refId,
              content: d.content,
              characterCount: d.characterCount,
              corpusVersion,
              genre: d.genre,
            })),
          )
          .onConflictDoNothing();
        await db
          .update(PipelineRuns)
          .set({ status: 'succeeded', doneDocs: selected.length, totalDocs: selected.length, finishedAt: new Date(), meta: { genreDist } })
          .where(eq(PipelineRuns.id, runId));
      });

      return { done: true, frozen: selected.length };
    } catch (err) {
      const message = String(err).slice(0, 1000);
      await step.do('mark-failed', async () => {
        await db.update(PipelineRuns).set({ status: 'failed', error: message, finishedAt: new Date() }).where(eq(PipelineRuns.id, runId));
      });
      return { failed: true };
    }
  }
}
