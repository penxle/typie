import { describe, expect, it } from 'vitest';
import {
  anchorMatchRate,
  categoryComplianceRate,
  cohenKappa,
  deriveWinRates,
  feedbackCountDistribution,
  flaggedRate,
  pairwiseFromRanking,
} from './aggregate.ts';
import type { PairVerdict } from './types.ts';

describe('pairwiseFromRanking', () => {
  const ranks = [
    { setId: 'x', rank: 1 },
    { setId: 'y', rank: 2 },
    { setId: 'z', rank: 2 },
  ];

  it('낮은 rank가 승리한다', () => {
    expect(pairwiseFromRanking(ranks, 'x', 'y')).toBe('a');
    expect(pairwiseFromRanking(ranks, 'y', 'x')).toBe('b');
  });

  it('동률은 tie', () => {
    expect(pairwiseFromRanking(ranks, 'y', 'z')).toBe('tie');
  });
});

describe('deriveWinRates', () => {
  it('후보별 vs V0 승패를 집계한다', () => {
    const entries = [
      {
        ranks: [
          { setId: 'v0', rank: 2 },
          { setId: 'c1s', rank: 1 },
          { setId: 'c2s', rank: 3 },
        ],
        v0SetId: 'v0',
        candidateSetIds: new Map([
          ['C1', 'c1s'],
          ['C2', 'c2s'],
        ]),
      },
      {
        ranks: [
          { setId: 'v0b', rank: 1 },
          { setId: 'c1sb', rank: 1 },
          { setId: 'c2sb', rank: 2 },
        ],
        v0SetId: 'v0b',
        candidateSetIds: new Map([
          ['C1', 'c1sb'],
          ['C2', 'c2sb'],
        ]),
      },
    ];
    const rates = deriveWinRates(entries);
    expect(rates.get('C1')).toEqual({ win: 1, tie: 1, loss: 0 });
    expect(rates.get('C2')).toEqual({ win: 0, tie: 0, loss: 2 });
  });
});

describe('cohenKappa', () => {
  it('완전 일치는 1', () => {
    const pairs: [PairVerdict, PairVerdict][] = [
      ['a', 'a'],
      ['b', 'b'],
      ['tie', 'tie'],
    ];
    expect(cohenKappa(pairs)).toBe(1);
  });

  it('우연 수준 일치는 0 근처', () => {
    const pairs: [PairVerdict, PairVerdict][] = [
      ['a', 'a'],
      ['a', 'b'],
      ['b', 'a'],
      ['b', 'b'],
    ];
    expect(cohenKappa(pairs)).toBeCloseTo(0, 5);
  });

  it('표본 없으면 NaN', () => {
    expect(cohenKappa([])).toBeNaN();
  });
});

describe('flaggedRate', () => {
  it('variant별 flagged/feedback 비율을 합산한다', () => {
    const entries = [
      { variantId: 'V0', feedbackCount: 10, flaggedCount: 2 },
      { variantId: 'V0', feedbackCount: 10, flaggedCount: 0 },
      { variantId: 'C1', feedbackCount: 5, flaggedCount: 1 },
    ];
    const rates = flaggedRate(entries);
    expect(rates.get('V0')).toBeCloseTo(0.1, 5);
    expect(rates.get('C1')).toBeCloseTo(0.2, 5);
  });
});

describe('anchorMatchRate', () => {
  it('variant별 matched/feedback 비율을 합산한다', () => {
    const entries = [
      { variantId: 'V0', matchedCount: 8, feedbackCount: 10 },
      { variantId: 'V0', matchedCount: 10, feedbackCount: 10 },
      { variantId: 'C1', matchedCount: 1, feedbackCount: 5 },
    ];
    const rates = anchorMatchRate(entries);
    expect(rates.get('V0')).toBeCloseTo(0.9, 5);
    expect(rates.get('C1')).toBeCloseTo(0.2, 5);
  });

  it('총 피드백이 0이면 NaN', () => {
    const rates = anchorMatchRate([{ variantId: 'V0', matchedCount: 0, feedbackCount: 0 }]);
    expect(rates.get('V0')).toBeNaN();
  });
});

describe('feedbackCountDistribution', () => {
  it('variant별 0건·총 세트를 집계한다', () => {
    const entries = [
      { variantId: 'V0', feedbackCount: 0 },
      { variantId: 'V0', feedbackCount: 11 },
      { variantId: 'V0', feedbackCount: 5 },
      { variantId: 'C1', feedbackCount: 20 },
    ];
    const dist = feedbackCountDistribution(entries);
    expect(dist.get('V0')).toEqual({ zero: 1, total: 3 });
    expect(dist.get('C1')).toEqual({ zero: 0, total: 1 });
  });
});

describe('categoryComplianceRate', () => {
  it('한글 짧은 라벨(2–10자, ASCII 문자 없음) 준수 비율', () => {
    expect(categoryComplianceRate(['맞춤법', '표현', null, 'grammar', '가'])).toBeCloseTo(0.4, 5);
  });

  it('빈 입력은 NaN', () => {
    expect(categoryComplianceRate([])).toBeNaN();
  });
});
