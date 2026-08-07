import type { RunUsage, UsageFold } from './types.ts';

// 모델 단가와 원가 환산. 승계 원본: `git show 'af4227815^:apps/eval/src/lib/domain/pricing.ts'`.
//
// 구 표는 실행당 입력·출력 한 쌍만 남는 원장 위에서 계산했지만(포함형 promptTokens), prism usage의
// fold는 네 축이 서로소다 — 그래서 표·환율·캐시 승수만 승계하고 계산은 새로 쓴다.

// cacheRead는 캐시에서 읽힌 입력 토큰의 단가. 대개 입력가의 10%다.
export type ModelPrice = { input: number; output: number; cacheRead?: number };
export type PriceTable = { models: Record<string, ModelPrice>; usdKrw: number };

// USD / 100만 토큰. 게이트웨이가 쓰는 모델 문자열을 그대로 키로 둔다.
// 출처와 확인 시점: 2026-07-26
//   Anthropic  platform.claude.com/docs/en/about-claude/pricing
//   Google     ai.google.dev/gemini-api/docs/pricing
//   OpenAI     developers.openai.com/api/docs/pricing (짧은 컨텍스트 기준)
//   xAI        docs.x.ai/developers/pricing (프롬프트 20만 토큰 미만 기준)
// 환율은 xe.com 중간환율(2026-07-22).
export const DEFAULT_PRICE_TABLE: PriceTable = {
  models: {
    'anthropic/claude-opus-5': { input: 5, output: 25, cacheRead: 0.5 },
    'anthropic/claude-fable-5': { input: 10, output: 50, cacheRead: 1 },
    // 소넷 5는 2026-08-31까지 도입가($2/$10), 이후 $3/$15로 오른다.
    'anthropic/claude-sonnet-5': { input: 2, output: 10, cacheRead: 0.2 },
    'anthropic/claude-haiku-4-5': { input: 1, output: 5, cacheRead: 0.1 },
    'google-vertex-ai/google/gemini-3.6-flash': { input: 1.5, output: 7.5, cacheRead: 0.15 },
    'google-vertex-ai/google/gemini-3.5-flash': { input: 1.5, output: 9, cacheRead: 0.15 },
    'google-vertex-ai/google/gemini-3.5-flash-lite': { input: 0.3, output: 2.5, cacheRead: 0.03 },
    'openai/gpt-5.6-sol': { input: 5, output: 30, cacheRead: 0.5 },
    'openai/gpt-5.6-terra': { input: 2.5, output: 15, cacheRead: 0.25 },
    'openai/gpt-5.6-luna': { input: 1, output: 6, cacheRead: 0.1 },
    'grok/grok-4.5': { input: 2, output: 6, cacheRead: 0.3 },
  },
  usdKrw: 1480,
};

// 캐시 쓰기는 입력가의 1.25배다(5분 TTL 기준). 읽기가 0.1배이므로 같은 접두부를 두 번만
// 읽어도 남는다 — 반대로 한 번 쓰고 읽지 못하면 손해다.
const CACHE_WRITE_MULTIPLIER = 1.25;

// fold의 네 축은 서로소다(usage.ts) — 어느 축도 다른 축에 포함되지 않으므로 빼내지 않고 각자의 단가로 친다.
// 표에 없는 모델이면 금액을 내지 않는다. 한쪽 단가로 뭉뚱그리면 몇 배씩 틀린다.
export const foldCost = (fold: UsageFold, table: PriceTable = DEFAULT_PRICE_TABLE): { krw: number } | null => {
  const price = table.models[`${fold.provider}/${fold.model}`];
  if (!price) return null;

  const usd =
    (fold.inputTokens * price.input +
      fold.cacheReadTokens * (price.cacheRead ?? price.input) +
      fold.cacheWriteTokens * price.input * CACHE_WRITE_MULTIPLIER +
      fold.outputTokens * price.output) /
    1_000_000;
  return { krw: usd * table.usdKrw };
};

// 부분합은 내지 않는다 — 단가를 모르는 fold가 하나라도 섞이면 남은 fold의 합은 총액이 아니라
// 총액의 일부이고, 그 수치를 '비용'으로 내걸면 실제보다 싸 보인다.
// complete=false는 아직 접히지 않은 fold가 있다는 뜻이다(usage.ts) — 그때 이 값은 하한이다.
export const sumCost = (usage: RunUsage | null, table: PriceTable = DEFAULT_PRICE_TABLE): { krw: number } | null => {
  if (!usage) return null;

  let krw = 0;
  for (const fold of usage.folds ?? []) {
    const cost = foldCost(fold, table);
    if (!cost) return null;
    krw += cost.krw;
  }
  return { krw };
};

export const formatKrw = (krw: number): string => {
  if (krw >= 10_000) return `${(krw / 10_000).toLocaleString('ko', { maximumFractionDigits: 1 })}만원`;
  return `${Math.round(krw).toLocaleString('ko')}원`;
};
