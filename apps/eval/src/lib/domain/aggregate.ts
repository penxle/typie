import type { PairVerdict } from './types.ts';

export const pairwiseFromRanking = (ranks: { setId: string; rank: number }[], aSetId: string, bSetId: string): PairVerdict => {
  const a = ranks.find((r) => r.setId === aSetId)?.rank ?? Infinity;
  const b = ranks.find((r) => r.setId === bSetId)?.rank ?? Infinity;
  if (a < b) return 'a';
  if (b < a) return 'b';
  return 'tie';
};

export const deriveWinRates = (
  entries: { ranks: { setId: string; rank: number }[]; v0SetId: string; candidateSetIds: Map<string, string> }[],
): Map<string, { win: number; tie: number; loss: number }> => {
  const rates = new Map<string, { win: number; tie: number; loss: number }>();
  for (const entry of entries) {
    for (const [variantId, setId] of entry.candidateSetIds) {
      const rate = rates.get(variantId) ?? { win: 0, tie: 0, loss: 0 };
      const verdict = pairwiseFromRanking(entry.ranks, setId, entry.v0SetId);
      if (verdict === 'a') rate.win++;
      else if (verdict === 'b') rate.loss++;
      else rate.tie++;
      rates.set(variantId, rate);
    }
  }
  return rates;
};

const categories: PairVerdict[] = ['a', 'b', 'tie'];

export const cohenKappa = (pairs: [PairVerdict, PairVerdict][]): number => {
  if (pairs.length === 0) return NaN;
  const n = pairs.length;
  const observed = pairs.filter(([x, y]) => x === y).length / n;
  let expected = 0;
  for (const c of categories) {
    const p1 = pairs.filter(([x]) => x === c).length / n;
    const p2 = pairs.filter(([, y]) => y === c).length / n;
    expected += p1 * p2;
  }
  if (expected === 1) return 1;
  return (observed - expected) / (1 - expected);
};

export const sanityPassRate = (verdicts: PairVerdict[]): number => {
  if (verdicts.length === 0) return NaN;
  return verdicts.filter((v) => v === 'tie').length / verdicts.length;
};

export const flaggedRate = (entries: { variantId: string; feedbackCount: number; flaggedCount: number }[]): Map<string, number> => {
  const totals = new Map<string, { feedbacks: number; flagged: number }>();
  for (const entry of entries) {
    const total = totals.get(entry.variantId) ?? { feedbacks: 0, flagged: 0 };
    total.feedbacks += entry.feedbackCount;
    total.flagged += entry.flaggedCount;
    totals.set(entry.variantId, total);
  }
  return new Map([...totals].map(([variantId, t]) => [variantId, t.feedbacks === 0 ? NaN : t.flagged / t.feedbacks]));
};

export const anchorMatchRate = (entries: { variantId: string; matchedCount: number; feedbackCount: number }[]): Map<string, number> => {
  const totals = new Map<string, { matched: number; feedbacks: number }>();
  for (const entry of entries) {
    const total = totals.get(entry.variantId) ?? { matched: 0, feedbacks: 0 };
    total.matched += entry.matchedCount;
    total.feedbacks += entry.feedbackCount;
    totals.set(entry.variantId, total);
  }
  return new Map([...totals].map(([variantId, t]) => [variantId, t.feedbacks === 0 ? NaN : t.matched / t.feedbacks]));
};

export const feedbackCountDistribution = (
  entries: { variantId: string; feedbackCount: number }[],
): Map<string, { zero: number; total: number }> => {
  const dist = new Map<string, { zero: number; total: number }>();
  for (const entry of entries) {
    const d = dist.get(entry.variantId) ?? { zero: 0, total: 0 };
    if (entry.feedbackCount === 0) d.zero++;
    d.total++;
    dist.set(entry.variantId, d);
  }
  return dist;
};

// score는 1-5 척도이며, rank는 6 - score로 변환해 기존 순위 기반 비교 경로를 그대로 재사용한다.
export const ranksFromScores = (scores: { setId: string; score: number }[]): { setId: string; rank: number }[] =>
  scores.map((s) => ({ setId: s.setId, rank: 6 - s.score }));

export const averageScores = (entries: { variantId: string; score: number }[]): Map<string, number> => {
  const sums = new Map<string, { total: number; count: number }>();
  for (const { variantId, score } of entries) {
    const acc = sums.get(variantId) ?? { total: 0, count: 0 };
    acc.total += score;
    acc.count += 1;
    sums.set(variantId, acc);
  }
  return new Map([...sums].map(([variantId, { total, count }]) => [variantId, total / count]));
};

export const categoryComplianceRate = (categories: (string | null)[]): number => {
  if (categories.length === 0) return NaN;
  const compliant = categories.filter((c) => c !== null && c.length >= 2 && c.length <= 10 && !/[a-z]/i.test(c)).length;
  return compliant / categories.length;
};
