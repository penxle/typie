import type { PrismReviewTierName } from '@typie/prism';

export const TIER_OPTIONS: { tier: PrismReviewTierName; label: string; time: string }[] = [
  { tier: 'low', label: '낮음', time: '몇 분' },
  { tier: 'medium', label: '보통', time: '이삼십 분' },
  { tier: 'high', label: '높음', time: '한 시간 남짓' },
];
