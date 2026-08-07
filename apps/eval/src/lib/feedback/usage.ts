import type { RunUsage } from './types.ts';

export type UsageTotals = {
  complete: boolean;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
};

// 합산 축은 네 개로 고정한다 — thinkingTokens는 어떤 축에도 섞지 않는다(이중 계상 금지).
export const sumUsage = (usage: RunUsage | null): UsageTotals | null => {
  if (!usage) return null;

  const total: UsageTotals = { complete: usage.complete, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0 };
  // 한 행의 결측이 표 전체를 500으로 떨구지 않게 folds 부재를 빈 목록으로 받는다.
  for (const fold of usage.folds ?? []) {
    total.inputTokens += fold.inputTokens;
    total.outputTokens += fold.outputTokens;
    total.cacheReadTokens += fold.cacheReadTokens;
    total.cacheWriteTokens += fold.cacheWriteTokens;
  }
  return total;
};
