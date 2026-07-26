import { eq, inArray } from 'drizzle-orm';
import { DEFAULT_PRICE_TABLE, parsePriceTable } from '$lib/domain/pricing.ts';
import { AnalysisPromptSets, PipelineRuns, PromptVariants, Settings, Variants } from './db/index.ts';
import type { PriceTable } from '$lib/domain/pricing.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

export const PRICE_SETTING_KEY = 'model_prices';

export const readPriceTable = async (db: Db): Promise<PriceTable> => {
  const [row] = await db.select({ value: Settings.value }).from(Settings).where(eq(Settings.key, PRICE_SETTING_KEY));
  if (!row) return DEFAULT_PRICE_TABLE;
  try {
    return parsePriceTable(JSON.parse(row.value)) ?? DEFAULT_PRICE_TABLE;
  } catch {
    return DEFAULT_PRICE_TABLE;
  }
};

export const writePriceTable = async (db: Db, table: PriceTable): Promise<void> => {
  await db
    .insert(Settings)
    .values({ key: PRICE_SETTING_KEY, value: JSON.stringify(table) })
    .onConflictDoUpdate({ target: Settings.key, set: { value: JSON.stringify(table) } });
};

const distinct = (models: (string | undefined)[]): string[] => [...new Set(models.filter((m): m is string => typeof m === 'string'))];

// 실행이 쓴 모델 목록. 실행 시각에 meta.models로 박아두지만, 그 전에 돈 실행은 프롬프트를
// 되짚어 알아낸다 — 그 사이 프롬프트가 수정됐다면 지금 값이라 어긋날 수 있다.
export const resolveRunModels = async (db: Db, runIds: string[]): Promise<Map<string, string[]>> => {
  const result = new Map<string, string[]>();
  if (runIds.length === 0) return result;

  const runs = await db
    .select({ id: PipelineRuns.id, variantId: PipelineRuns.variantId, meta: PipelineRuns.meta })
    .from(PipelineRuns)
    .where(inArray(PipelineRuns.id, runIds));

  const needSet: { runId: string; promptSetId: string }[] = [];
  const needVariant: { runId: string; variantId: string }[] = [];

  for (const run of runs) {
    const meta = (run.meta ?? {}) as { models?: unknown; promptSetId?: unknown };
    if (Array.isArray(meta.models)) {
      result.set(run.id, distinct(meta.models as string[]));
      continue;
    }
    if (typeof meta.promptSetId === 'string') {
      needSet.push({ runId: run.id, promptSetId: meta.promptSetId });
      continue;
    }
    if (run.variantId) needVariant.push({ runId: run.id, variantId: run.variantId });
  }

  if (needSet.length > 0) {
    const sets = await db
      .select({ id: AnalysisPromptSets.id, content: AnalysisPromptSets.content })
      .from(AnalysisPromptSets)
      .where(inArray(AnalysisPromptSets.id, [...new Set(needSet.map((n) => n.promptSetId))]));
    const byId = new Map(sets.map((s) => [s.id, s.content]));
    for (const need of needSet) {
      const content = byId.get(need.promptSetId);
      result.set(need.runId, content ? distinct(Object.values(content).map((p) => p.model)) : []);
    }
  }

  if (needVariant.length > 0) {
    const variantIds = [...new Set(needVariant.map((n) => n.variantId))];
    const variants = await db
      .select({ id: Variants.id, promptVariantId: Variants.promptVariantId })
      .from(Variants)
      .where(inArray(Variants.id, variantIds));
    const promptVariantIds = variants.map((v) => v.promptVariantId).filter((id): id is string => id !== null);
    const prompts =
      promptVariantIds.length > 0
        ? await db
            .select({ id: PromptVariants.id, content: PromptVariants.content })
            .from(PromptVariants)
            .where(inArray(PromptVariants.id, promptVariantIds))
        : [];
    const contentById = new Map(prompts.map((p) => [p.id, p.content]));
    const promptVariantByVariant = new Map(variants.map((v) => [v.id, v.promptVariantId]));

    for (const need of needVariant) {
      const promptVariantId = promptVariantByVariant.get(need.variantId);
      const content = promptVariantId ? contentById.get(promptVariantId) : undefined;
      result.set(need.runId, content ? distinct(Object.values(content).map((p) => p.model)) : []);
    }
  }

  for (const run of runs) {
    if (!result.has(run.id)) result.set(run.id, []);
  }
  return result;
};
