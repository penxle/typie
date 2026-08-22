import type { PrismReviewTierName } from '@typie/prism';

export const TIER_OPTIONS: { tier: PrismReviewTierName; label: string; time: string }[] = [
  { tier: 'low', label: '빠른 검토', time: '몇 분' },
  { tier: 'medium', label: '일반 검토', time: '삼십 분 정도' },
  { tier: 'high', label: '심층 검토', time: '한 시간 남짓' },
];
