import { and, eq, inArray } from 'drizzle-orm';
import { planPersonalIntake } from '$lib/domain/personal-corpus.ts';
import { estimateCost } from '$lib/domain/pricing.ts';
import { PERSONAL_CORPUS_VERSION } from '$lib/domain/types.ts';
import { Documents, FeedbackSets, PipelineRuns } from './db/index.ts';
import { readPriceTable, resolveRunModels } from './pricing.ts';
import { spawnAnalysisRun } from './runs.ts';
import type { Cost } from '$lib/domain/pricing.ts';
import type { createDb } from './db/index.ts';
import type { InternalApi } from './internal-api.ts';

type Db = ReturnType<typeof createDb>;
type Env = App.Platform['env'];

// 개인 열람용 글은 코퍼스와 같은 표 안에 살지만 kind로 갈린다. 코퍼스 버전은 하나로 고정하고
// 실행할 때마다 documentIds로 범위를 좁힌다 — 버전을 새로 파면 실행 모달의 코퍼스 목록이
// 사람 수만큼 불어나고, 한 버전에 몰아넣고 범위를 안 주면 들일 때마다 전체가 다시 돈다.
export const intakePersonalDocuments = async (
  db: Db,
  api: InternalApi,
  requestedIds: string[],
): Promise<{ accepted: { id: string; refId: string; characterCount: number }[]; rejected: { refId: string; reason: string }[] }> => {
  const existing = await db
    .select({ refId: Documents.refId })
    .from(Documents)
    .where(and(eq(Documents.corpusVersion, PERSONAL_CORPUS_VERSION), inArray(Documents.refId, requestedIds)));

  // 공개 관문을 먼저 통과시킨 id만 추출로 넘긴다. extract는 공개 여부를 보지 않으므로
  // 순서를 뒤집으면 비공개 글의 본문을 뽑아 놓고 나서야 버리게 된다.
  const publicRows = await api.publicTexts(requestedIds);
  const publicIds = publicRows.map((r) => r.documentId);
  const extracted = publicIds.length > 0 ? await api.extract(publicIds) : [];

  const plan = planPersonalIntake({
    requestedIds,
    publicIds,
    existingRefIds: existing.map((e) => e.refId),
    extracted,
  });

  const rows = plan.accepted.map((doc) => ({
    id: crypto.randomUUID(),
    refId: doc.refId,
    content: doc.prose,
    characterCount: doc.characterCount,
    corpusVersion: PERSONAL_CORPUS_VERSION,
    kind: 'personal' as const,
    genre: null,
  }));

  // D1은 문장당 바인딩 파라미터 100개 제한 — 8컬럼이라 10행씩 나눈다(표집 경로와 같은 이유).
  for (let i = 0; i < rows.length; i += 10) {
    await db
      .insert(Documents)
      .values(rows.slice(i, i + 10))
      .onConflictDoNothing();
  }

  return {
    accepted: rows.map((r) => ({ id: r.id, refId: r.refId, characterCount: r.characterCount })),
    rejected: plan.rejected,
  };
};

export const spawnPersonalRun = async (
  db: Db,
  env: Env,
  input: { promptSetId: string; documentIds: string[] },
): Promise<{ runId: string; spawnedCount: number; failedCount: number } | { error: string }> =>
  spawnAnalysisRun(db, env, {
    promptSetId: input.promptSetId,
    corpusVersion: PERSONAL_CORPUS_VERSION,
    documentIds: input.documentIds,
  });

export type PersonalRead = {
  setId: string;
  refId: string;
  characterCount: number;
  runId: string;
  runStatus: string;
  cost: Cost;
  tokens: number;
  createdAt: string;
};

// 열람 링크 목록. 세트가 생긴 글만 나온다 — 실행 중인 글은 아직 보여줄 것이 없다.
export const listPersonalReads = async (db: Db): Promise<PersonalRead[]> => {
  const rows = await db
    .select({
      setId: FeedbackSets.id,
      refId: Documents.refId,
      characterCount: Documents.characterCount,
      runId: PipelineRuns.id,
      runStatus: PipelineRuns.status,
      promptTokens: PipelineRuns.promptTokens,
      completionTokens: PipelineRuns.completionTokens,
      cachedTokens: PipelineRuns.cachedTokens,
      createdAt: PipelineRuns.createdAt,
    })
    .from(FeedbackSets)
    .innerJoin(Documents, eq(Documents.id, FeedbackSets.documentId))
    .innerJoin(PipelineRuns, eq(PipelineRuns.id, FeedbackSets.runId))
    .where(eq(Documents.kind, 'personal'));

  const priceTable = await readPriceTable(db);
  const modelsByRun = await resolveRunModels(db, [...new Set(rows.map((r) => r.runId))]);

  return rows
    .map((row) => ({
      setId: row.setId,
      refId: row.refId,
      characterCount: row.characterCount,
      runId: row.runId,
      runStatus: row.runStatus,
      tokens: row.promptTokens + row.completionTokens,
      cost: estimateCost(
        {
          promptTokens: row.promptTokens,
          completionTokens: row.completionTokens,
          cachedTokens: row.cachedTokens,
          models: modelsByRun.get(row.runId) ?? [],
        },
        priceTable,
      ),
      createdAt: row.createdAt.toISOString(),
    }))
    .toSorted((a, b) => b.createdAt.localeCompare(a.createdAt));
};
