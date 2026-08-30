import { describe, expect, it } from 'vitest';
import { stageIntroducedIn, TIER_STAGES } from './stages.ts';
import { DELIVERABLES, TIER_OPTIONS, tierCovers } from './tiers.ts';
import type { PrismReviewTierName } from '@typie/prism';

const TIERS: PrismReviewTierName[] = ['low', 'medium', 'high'];

describe('tiers', () => {
  it('단계가 처음 열리는 티어는 티어별 단계표와 어긋나지 않는다', () => {
    for (const stage of TIER_STAGES.high) {
      const from = stageIntroducedIn(stage);
      for (const tier of TIERS) {
        expect(TIER_STAGES[tier].includes(stage)).toBe(tierCovers(tier, from));
      }
    }
  });

  it('총평은 일반부터, 강점·판정 계열은 심층부터 열린다', () => {
    const openIn = (tier: PrismReviewTierName) => DELIVERABLES.filter((item) => tierCovers(tier, item.from)).map((item) => item.label);

    expect(openIn('low')).toEqual(['문장별 피드백']);
    expect(openIn('medium')).toEqual([
      '문장별 피드백',
      '이 글이 어떻게 읽혔는지',
      '지난 회차보다 나아진 점',
      '주로 반복되는 문제',
      '어디부터 고치면 좋은지',
    ]);
    expect(openIn('high')).toHaveLength(DELIVERABLES.length);
  });

  it('목록은 열리는 티어 순으로 서서 티어를 바꿔도 재배열되지 않는다', () => {
    const ranks = DELIVERABLES.map((item) => TIERS.indexOf(item.from));
    expect(ranks).toEqual(ranks.toSorted((a, b) => a - b));
  });

  it('티어마다 용도 문구가 있다', () => {
    expect(TIER_OPTIONS.map((option) => option.tier)).toEqual(TIERS);
    for (const option of TIER_OPTIONS) expect(option.use).not.toBe('');
  });
});
