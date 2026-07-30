import { eq } from 'drizzle-orm';
import { DEFAULT_PRICE_TABLE, estimateCost, parsePriceTable } from '$lib/domain/pricing.ts';
import { Settings } from '../../../core/db.ts';
import type { Cost, PriceTable } from '$lib/domain/pricing.ts';
import type { Usage } from '../../../core/contracts.ts';
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

export type CallUsageRow = { phase: string; usage: Usage; durationMs: number };
export type PipelinePhase = PhaseUsageRow & { durationMs: number };

// 호출별 원장(call_usage)을 단계로 합친다. phase_usage는 시도를 넘어 누적되어 캐시를 비우고
// 재실행하면 실패한 시도의 비용까지 합산돼 보이지만, 원장 행은 호출당 하나라 이 합이 곧 마지막
// 성공 경로의 파이프라인 1회분이다. 화면 회계는 이 함수 하나로만 나온다 — 도입 이전 실행은
// 마이그레이션이 phase_usage를 합성 행으로 백필해 같은 길로 들어온다.
export const pipelineUsage = (rows: CallUsageRow[]): PipelinePhase[] => {
  const byPhase = new Map<string, PipelinePhase>();
  for (const row of rows) {
    const acc = byPhase.get(row.phase) ?? {
      phase: row.phase,
      calls: 0,
      promptTokens: 0,
      completionTokens: 0,
      cachedTokens: 0,
      cacheWriteTokens: 0,
      durationMs: 0,
    };
    acc.calls += row.usage.calls;
    acc.promptTokens += row.usage.promptTokens;
    acc.completionTokens += row.usage.completionTokens;
    acc.cachedTokens += row.usage.cachedTokens;
    acc.cacheWriteTokens += row.usage.cacheWriteTokens;
    acc.durationMs += row.durationMs;
    byPhase.set(row.phase, acc);
  }
  // 사용량도 시간도 없는 단계는 표에 빈 줄만 만든다.
  return [...byPhase.values()].filter((row) => row.calls > 0 || row.promptTokens > 0 || row.completionTokens > 0 || row.durationMs > 0);
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
