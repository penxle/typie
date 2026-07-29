import { eq } from 'drizzle-orm';
import { estimateCost } from '$lib/domain/pricing.ts';
import { AnalysisPromptSets, AnalysisStageUsage, PipelineRuns } from './db/index.ts';
import { readPriceTable } from './pricing.ts';
import type { Cost } from '$lib/domain/pricing.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

export type StageUsage = {
  stage: string;
  calls: number;
  promptTokens: number;
  completionTokens: number;
  cachedTokens: number;
  cacheWriteTokens: number;
  model: string | null;
  cost: Cost;
};

// 파이프라인이 도는 순서. 표에서 비용 순으로 재정렬하면 어느 단계가 어디에 있는지 매번
// 다시 읽어야 한다 — 순서를 고정해 두면 실행끼리 견주기 쉽다.
const STAGE_ORDER = [
  'survey',
  'background',
  'review',
  'dedupe',
  'verify',
  'research',
  'plan-draft',
  'plan-revise-0',
  'plan-revise-1',
  'plan-revise-2',
  'plan',
  'planReview',
  'execute',
  'local',
  'compose',
  'composeReview',
];

// 런타임 단계명을 세트의 프롬프트 키로 정규화한다 — 계획 초안·수정 라운드는 전부 plan 프롬프트로
// 돌았고, 에디토리얼 레거시 실행의 'plan' 행은 검수(OpenAI) 토큰이다(이후 planReview로 기록 변경).
const setKeyOf = (stage: string, content: Record<string, unknown>): string => {
  if (stage === 'plan-draft' || stage.startsWith('plan-revise')) return 'plan';
  if (stage === 'plan' && 'research' in content) return 'planReview';
  return stage;
};

/**
 * 실행의 단계별 사용량과 금액.
 *
 * 단계마다 모델이 다를 수 있어 금액도 단계별로 낸다 — 실행 단위 estimateCost는 모델이 둘 이상이면
 * '혼합'으로 물러나 금액을 내지 않는다. 모델은 실행이 쓴 프롬프트 세트에서 되짚으므로, 그 사이
 * 세트가 수정됐다면 어긋날 수 있다(세트는 덮어쓰지 않는 것이 관례라 실제로는 드물다).
 */
export const readStageUsage = async (db: Db, runId: string): Promise<StageUsage[]> => {
  const rows = await db
    .select({
      stage: AnalysisStageUsage.stage,
      calls: AnalysisStageUsage.calls,
      promptTokens: AnalysisStageUsage.promptTokens,
      completionTokens: AnalysisStageUsage.completionTokens,
      cachedTokens: AnalysisStageUsage.cachedTokens,
      cacheWriteTokens: AnalysisStageUsage.cacheWriteTokens,
    })
    .from(AnalysisStageUsage)
    .where(eq(AnalysisStageUsage.runId, runId));
  if (rows.length === 0) return [];

  // 문서별로 쌓인 것을 단계로 합친다.
  const byStage = new Map<string, Omit<StageUsage, 'model' | 'cost'>>();
  for (const row of rows) {
    const acc = byStage.get(row.stage) ?? {
      stage: row.stage,
      calls: 0,
      promptTokens: 0,
      completionTokens: 0,
      cachedTokens: 0,
      cacheWriteTokens: 0,
    };
    acc.calls += row.calls;
    acc.promptTokens += row.promptTokens;
    acc.completionTokens += row.completionTokens;
    acc.cachedTokens += row.cachedTokens;
    acc.cacheWriteTokens += row.cacheWriteTokens;
    byStage.set(row.stage, acc);
  }

  const [run] = await db.select({ meta: PipelineRuns.meta }).from(PipelineRuns).where(eq(PipelineRuns.id, runId)).limit(1);
  const promptSetId = (run?.meta as { promptSetId?: unknown } | null)?.promptSetId;
  let modelByStage = new Map<string, string>();
  let setContent: Record<string, unknown> = {};
  if (typeof promptSetId === 'string') {
    const [set] = await db
      .select({ content: AnalysisPromptSets.content })
      .from(AnalysisPromptSets)
      .where(eq(AnalysisPromptSets.id, promptSetId))
      .limit(1);
    if (set) {
      setContent = set.content;
      modelByStage = new Map(Object.entries(set.content).map(([stage, prompt]) => [stage, prompt.model]));
    }
  }

  const priceTable = await readPriceTable(db);

  return [...byStage.values()]
    .map((acc) => {
      const model = modelByStage.get(setKeyOf(acc.stage, setContent)) ?? null;
      return { ...acc, model, cost: estimateCost({ ...acc, models: model ? [model] : [] }, priceTable) };
    })
    .toSorted((a, b) => {
      const ai = STAGE_ORDER.indexOf(a.stage);
      const bi = STAGE_ORDER.indexOf(b.stage);
      return (ai === -1 ? STAGE_ORDER.length : ai) - (bi === -1 ? STAGE_ORDER.length : bi) || a.stage.localeCompare(b.stage);
    });
};
