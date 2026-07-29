import { eq } from 'drizzle-orm';
import { DEFAULT_PRICE_TABLE, estimateCost, parsePriceTable } from '$lib/domain/pricing.ts';
import { Settings } from '../../../core/db.ts';
import type { Cost, PriceTable } from '$lib/domain/pricing.ts';
import type { Db } from '../../../core/db.ts';

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

export type PhaseUsageRow = {
  phase: string;
  calls: number;
  promptTokens: number;
  completionTokens: number;
  cachedTokens: number;
  cacheWriteTokens: number;
};

// 프롬프트 묶음이 단계마다 모델을 정한다 — 어느 토큰이 어느 모델 것인지는 이 표로만 되돌린다.
// 열 자체는 자유 JSON이라 모양을 믿지 않고 꺼낸다.
export type PromptContent = Record<string, unknown>;

export const modelOf = (content: PromptContent | null, phase: string): string | null => {
  const model = (content?.[phase] as { model?: unknown } | undefined)?.model;
  return typeof model === 'string' && model ? model : null;
};

// 단계마다 모델이 다를 수 있어 금액도 단계별로 낸다 — 실행 단위 estimateCost는 모델이 둘 이상이면
// '혼합'으로 떨어져 금액을 내지 못한다.
export const phaseCosts = (
  usage: PhaseUsageRow[],
  content: PromptContent | null,
  table: PriceTable,
): (PhaseUsageRow & { model: string | null; cost: Cost })[] =>
  usage.map((row) => {
    const model = modelOf(content, row.phase);
    return { ...row, model, cost: estimateCost({ ...row, models: model ? [model] : [] }, table) };
  });

export const totalUsage = (usage: PhaseUsageRow[]) =>
  usage.reduce(
    (sum, row) => ({
      promptTokens: sum.promptTokens + row.promptTokens,
      completionTokens: sum.completionTokens + row.completionTokens,
      cachedTokens: sum.cachedTokens + row.cachedTokens,
      cacheWriteTokens: sum.cacheWriteTokens + row.cacheWriteTokens,
    }),
    { promptTokens: 0, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0 },
  );

export const runCost = (usage: PhaseUsageRow[], content: PromptContent | null, table: PriceTable): Cost =>
  estimateCost(
    {
      ...totalUsage(usage),
      models: [...new Set(usage.map((row) => modelOf(content, row.phase)).filter((m): m is string => m !== null))],
    },
    table,
  );
