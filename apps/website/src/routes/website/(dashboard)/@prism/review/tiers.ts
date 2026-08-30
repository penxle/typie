import { PRISM_REVIEW_TIERS } from '@typie/prism';
import type { PrismReviewTierName } from '@typie/prism';

export const TIER_OPTIONS: { tier: PrismReviewTierName; label: string; time: string; use: string }[] = [
  { tier: 'low', label: '빠른 검토', time: '몇 분 정도', use: '걸리는 자리만 빠르게 볼 수 있어요' },
  { tier: 'medium', label: '일반 검토', time: '삼십 분 내외', use: '무엇부터 고쳐야 할지 받아볼 수 있어요' },
  { tier: 'high', label: '심층 검토', time: '한 시간 남짓', use: '무엇이 잘됐고 무엇이 약한지 알 수 있어요' },
];

// 결과에 실리는 것들. from은 코드에서 파생할 수 없다 — 총평 절의 유무는 prism 워크플로가 티어마다 다르게
// 정하는 것이라 여기서 선언하고 테스트로 고정한다. 배열 순서가 곧 열리는 차례라서, 티어를 바꿔도 목록은
// 제자리에 서고 표시만 뒤집힌다.
export type Deliverable = { label: string; from: PrismReviewTierName; followupOnly?: boolean };

export const DELIVERABLES: Deliverable[] = [
  { label: '문장별 피드백', from: 'low' },
  { label: '이 글이 어떻게 읽혔는지', from: 'medium' },
  { label: '지난 회차보다 나아진 점', from: 'medium', followupOnly: true },
  { label: '주로 반복되는 문제', from: 'medium' },
  { label: '어디부터 고치면 좋은지', from: 'medium' },
  { label: '이 글의 강점과 약점', from: 'high' },
  { label: '잘 쓰인 부분과 그 이유', from: 'high' },
  { label: '더 좋아질 수 있는 부분', from: 'high' },
];

export const tierCovers = (tier: PrismReviewTierName, from: PrismReviewTierName): boolean =>
  PRISM_REVIEW_TIERS.indexOf(tier) >= PRISM_REVIEW_TIERS.indexOf(from);

export const tierLabelOf = (tier: PrismReviewTierName): string => TIER_OPTIONS.find((option) => option.tier === tier)?.label ?? tier;
