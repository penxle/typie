import type { RunUsage, UsageFold } from './types.ts';

// 원가 환산과 표기. 단가표의 정본은 prism이다(prism src/pricing.ts) — eval은 그 표를 API(GET /pricing,
// server/prism.ts fetchPriceTable)로 받아 이 함수들에 먹인다(dash는 같은 표를 코드로 직접 참조한다).
// 여기의 형은 그 와이어의 사영이라 prism 쪽 선언과 구조가 같아야 한다. 표 부재(수신 실패)는 단가 미상과
// 같은 취급이다 — 금액을 지어내지 않고 전부 미상(null)으로 강등한다.
//
// cacheRead는 캐시에서 읽힌 입력 토큰의 단가(대개 입력가의 10%). cacheWrite는 캐시에 써진 입력 토큰의
// 단가다 — 입력가 × TTL 승수(5분 1.25배, 1시간 2배)의 산출 규칙과 근거는 정본 쪽에 있다.
export type ModelPrice = { input: number; output: number; cacheRead?: number; cacheWrite: number };
export type PriceTable = { models: Record<string, ModelPrice>; usdKrw: number };

// fold의 네 축은 서로소다(usage.ts) — 어느 축도 다른 축에 포함되지 않으므로 빼내지 않고 각자의 단가로 친다.
// 표에 없는 모델이면 금액을 내지 않는다. 한쪽 단가로 뭉뚱그리면 몇 배씩 틀린다.
export const foldCost = (fold: UsageFold, table: PriceTable | null): { krw: number } | null => {
  if (!table) return null;
  const price = table.models[`${fold.provider}/${fold.model}`];
  if (!price) return null;

  const usd =
    (fold.inputTokens * price.input +
      fold.cacheReadTokens * (price.cacheRead ?? price.input) +
      fold.cacheWriteTokens * price.cacheWrite +
      fold.outputTokens * price.output) /
    1_000_000;
  return { krw: usd * table.usdKrw };
};

// 부분합은 내지 않는다 — 단가를 모르는 fold가 하나라도 섞이면 남은 fold의 합은 총액이 아니라
// 총액의 일부이고, 그 수치를 '비용'으로 내걸면 실제보다 싸 보인다.
// complete=false는 아직 접히지 않은 fold가 있다는 뜻이다(usage.ts) — 그때 이 값은 하한이다.
export const sumCost = (usage: RunUsage | null, table: PriceTable | null): { krw: number } | null => {
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
