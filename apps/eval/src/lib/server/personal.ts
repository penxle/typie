import { and, eq, inArray, sql } from 'drizzle-orm';
import { planPersonalIntake } from '$lib/domain/personal-corpus.ts';
import { estimateCost } from '$lib/domain/pricing.ts';
import { PERSONAL_CORPUS_VERSION } from '$lib/domain/types.ts';
import { AnalysisPromptSets, Documents, FeedbackSets, PipelineRuns } from './db/index.ts';
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
): Promise<{
  accepted: { id: string; refId: string; characterCount: number }[];
  reused: { id: string; refId: string; characterCount: number }[];
  rejected: { refId: string; reason: string }[];
}> => {
  const existing = await db
    .select({ id: Documents.id, refId: Documents.refId, characterCount: Documents.characterCount })
    .from(Documents)
    .where(and(eq(Documents.corpusVersion, PERSONAL_CORPUS_VERSION), inArray(Documents.refId, requestedIds)));
  const existingByRef = new Map(existing.map((e) => [e.refId, e]));

  // 공개 여부를 묻지 않는다 — 비공개·성인글도 들어온다. 이 경로는 작성자 본인이 자기 글의
  // 피드백을 읽으려고 쓰는 자리이고, 어드민 전용이다.
  //
  // 이 동작은 /internal/corpus/extract가 documentId로만 조회한다는 사실에 기대고 있다.
  // 그 엔드포인트에 가시성 조건이 붙으면 이 경로가 조용히 막히므로, 그때는 이 용도의
  // 엔드포인트를 따로 두어야 한다. 표집 코퍼스는 여전히 /corpus/texts의 공개 관문을 쓴다.
  //
  // 이미 있는 글은 본문을 다시 뽑지 않는다 — 내용은 그대로이고 추출 호출만 낭비된다.
  const toExtract = requestedIds.filter((id) => !existingByRef.has(id));
  const extracted = toExtract.length > 0 ? await api.extract(toExtract) : [];

  const plan = planPersonalIntake({
    requestedIds,
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
    // 이미 들여온 글은 그대로 실행 대상에 넣는다. 같은 글을 다른 프롬프트 세트로 다시 돌려
    // 견주는 것이 이 기능의 주된 쓰임이다.
    reused: plan.reused.flatMap((refId) => {
      const doc = existingByRef.get(refId);
      return doc ? [{ id: doc.id, refId: doc.refId, characterCount: doc.characterCount }] : [];
    }),
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
  // 한 글에 세트가 여럿 생긴다 — 어느 프롬프트로 만든 것인지 없으면 링크를 구분할 수 없다.
  promptSet: string;
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
      promptSetId: sql<string | null>`json_extract(${PipelineRuns.meta}, '$.promptSetId')`,
      createdAt: PipelineRuns.createdAt,
    })
    .from(FeedbackSets)
    .innerJoin(Documents, eq(Documents.id, FeedbackSets.documentId))
    .innerJoin(PipelineRuns, eq(PipelineRuns.id, FeedbackSets.runId))
    .where(eq(Documents.kind, 'personal'));

  const priceTable = await readPriceTable(db);
  const modelsByRun = await resolveRunModels(db, [...new Set(rows.map((r) => r.runId))]);
  const setIds = [...new Set(rows.map((r) => r.promptSetId).filter((id): id is string => id !== null))];
  const promptSets =
    setIds.length > 0
      ? await db
          .select({ id: AnalysisPromptSets.id, label: AnalysisPromptSets.label })
          .from(AnalysisPromptSets)
          .where(inArray(AnalysisPromptSets.id, setIds))
      : [];
  const labelById = new Map(promptSets.map((p) => [p.id, p.label]));

  return rows
    .map((row) => ({
      setId: row.setId,
      refId: row.refId,
      promptSet: (row.promptSetId ? labelById.get(row.promptSetId) : null) ?? '?',
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
