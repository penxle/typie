import { WorkflowEntrypoint } from 'cloudflare:workers';
import { eq, inArray } from 'drizzle-orm';
import { classifyCorpusDocument } from './corpus-filter.ts';
import { createDb, Documents, inChunks, readCache, Samplings, writeCache } from './db.ts';
import { createInternalApi } from './internal-api.ts';
import { createOpenAI } from './openai.ts';
import {
  candidateLimitFor,
  capPerAuthor,
  excludeExisting,
  fillQuotas,
  pickLiteraryDocs,
  selectSuccessfulExtracts,
  stratifySelection,
} from './sampling-select.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { Db } from './db.ts';
import type { FlowEnv, SamplingParams } from './index.ts';
import type { Classified, LiteraryDoc, SelectedDocument } from './sampling-select.ts';

// 심사는 코퍼스 품질의 마지막 관문이다. 여기서 놓친 원고는 오라클 비용과 평가자 시간을 함께 버린다.
const CLASSIFY_MODEL = 'anthropic/claude-opus-5';
const CLASSIFY_BATCH = 8;
const EXTRACT_BATCH = 5;
// api 응답 크기(문서당 최대 30KB)와 D1 바인딩 파라미터(행당 2개, 100 제한) 양쪽에 여유.
const TEXTS_BATCH = 20;
const LLM_STEP = { retries: { limit: 2, delay: '10 seconds' as const, backoff: 'exponential' as const }, timeout: '5 minutes' as const };

const candidateKey = (documentId: string): string => `candidate/${documentId}`;

type SamplingPhase = 'candidates' | 'classify' | 'extract' | 'freeze';

const setPhase = async (db: Db, runId: string, phase: SamplingPhase): Promise<void> => {
  await db.update(Samplings).set({ phase }).where(eq(Samplings.id, runId));
};

export class SamplingWorkflow extends WorkflowEntrypoint<FlowEnv, SamplingParams> {
  async run(event: WorkflowEvent<SamplingParams>, step: WorkflowStep) {
    const { runId, size } = event.payload;
    const db = createDb(this.env.DB);
    const api = createInternalApi(this.env.INTERNAL_API_BASE, this.env.INTERNAL_API_KEY);
    // provider/model 명명('anthropic/claude-opus-5')은 compat 경로만 받는다 — anthropic 전용
    // 경로에 붙이면 전 호출이 404로 죽고, 아래 catch가 삼켜 후보 전멸로만 나타난다(2026-07-30 실측).
    const openai = createOpenAI(this.env.CLOUDFLARE_API_KEY, this.env.CLOUDFLARE_AIGATEWAY_COMPAT_URL);

    try {
      const candidatePairs = await step.do('candidates', async () => {
        await setPhase(db, runId, 'candidates');
        const candidates = await api.candidates({ limit: candidateLimitFor(size) });
        const existing = await inChunks(
          candidates.map((c) => c.documentId),
          (chunk) => db.select({ refId: Documents.refId }).from(Documents).where(inArray(Documents.refId, chunk)),
        );
        const fresh = excludeExisting(candidates, new Set(existing.map((e) => e.refId)));
        return fresh.map((c) => ({ documentId: c.documentId, userId: c.userId }));
      });
      const candidateIds = candidatePairs.map((c) => c.documentId);
      const authorByDoc = new Map(candidatePairs.map((c) => [c.documentId, c.userId]));

      for (let t = 0; t < candidateIds.length; t += TEXTS_BATCH) {
        const batchIds = candidateIds.slice(t, t + TEXTS_BATCH);
        await step.do(`texts-${t}`, LLM_STEP, async () => {
          const texts = await api.texts(batchIds);
          if (texts.length === 0) return;
          for (const row of texts) {
            await writeCache(db, runId, candidateKey(row.documentId), { text: row.text });
          }
        });
      }

      const literaryDocs: LiteraryDoc[] = [];
      // 개별 심사 오류는 삼키되 계수한다 — 계수가 없으면 계통 장애(전 호출 404 등)가
      // "후보 전멸"로만 보여 원인을 고고학으로 찾아야 한다.
      let classifyErrors = 0;
      await step.do('phase-classify', () => setPhase(db, runId, 'classify'));
      for (let b = 0; b < candidateIds.length; b += CLASSIFY_BATCH) {
        const batchIds = candidateIds.slice(b, b + CLASSIFY_BATCH);
        const { found, errored } = await step.do(`classify-${b}`, LLM_STEP, async () => {
          const classified = await Promise.all(
            batchIds.map(async (documentId): Promise<Classified> => {
              const cached = await readCache<{ text: string }>(db, runId, candidateKey(documentId));
              const text = cached?.text ?? '';
              try {
                // 앞부분만 잘라 넘기면 묶음·후기·복수 엔딩을 놓친다 — 전문을 주고 발췌는 심사기가 정한다.
                const result = await classifyCorpusDocument(openai, CLASSIFY_MODEL, text);
                return { candidate: { documentId, characterCount: 0, userId: authorByDoc.get(documentId) ?? '' }, ...result };
              } catch {
                return {
                  candidate: { documentId, characterCount: 0, userId: authorByDoc.get(documentId) ?? '' },
                  kind: 'error',
                  genre: 'etc',
                  narrative: false,
                  singleWork: false,
                  selfContained: false,
                  original: false,
                };
              }
            }),
          );
          return { found: pickLiteraryDocs(classified), errored: classified.filter((c) => c.kind === 'error').length };
        });
        literaryDocs.push(...found);
        classifyErrors += errored;
      }

      const { genreDist, allocation, picks } = await step.do('select-strata', () =>
        Promise.resolve(stratifySelection(capPerAuthor(literaryDocs, authorByDoc), size)),
      );

      await step.do('phase-extract', async () => {
        await setPhase(db, runId, 'extract');
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
      }

      const selected = fillQuotas(
        extracted.map((d) => ({ ...d, genre: genreByRef.get(d.refId) ?? 'etc' })),
        allocation,
        size,
      );

      if (selected.length < size) {
        throw new Error(
          `insufficient documents after extraction: ${selected.length}/${size} (후보 ${candidateIds.length}, 심사 오류 ${classifyErrors})`,
        );
      }

      await step.do('freeze', async () => {
        await setPhase(db, runId, 'freeze');
        const rows = selected.map((d) => ({
          id: d.id,
          refId: d.refId,
          content: d.content,
          characterCount: d.characterCount,
          kind: 'sampled' as const,
          genre: d.genre,
          samplingId: runId,
        }));
        // D1은 문장당 바인딩 파라미터 100개 제한 — 8컬럼 × 12행 = 96이 상한이라 10행씩 나눈다.
        for (let i = 0; i < rows.length; i += 10) {
          await db
            .insert(Documents)
            .values(rows.slice(i, i + 10))
            .onConflictDoNothing();
        }
        await db.update(Samplings).set({ status: 'done', phase: null, finishedAt: new Date() }).where(eq(Samplings.id, runId));
        console.warn(`sampling ${runId}: 장르 분포 ${JSON.stringify(genreDist)}`);
      });

      return { done: true, frozen: selected.length };
    } catch (err) {
      const message = String(err).slice(0, 1000);
      await step.do('mark-failed', async () => {
        await db
          .update(Samplings)
          .set({ status: 'failed', phase: null, error: message, finishedAt: new Date() })
          .where(eq(Samplings.id, runId));
      });
      throw err;
    }
  }
}
