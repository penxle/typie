import { TIER_OPTIONS } from './tiers.ts';
import type { DataOf } from '@mearie/svelte';
import type { WorkflowStatus } from '@typie/prism';
import type { DashboardLayout_PrismReviewPassage_Query } from '$mearie';

export type ReviewRound = DataOf<DashboardLayout_PrismReviewPassage_Query>['prismSession']['reviewRounds'][number];

// 총평 모달이 회차에서 실제로 읽는 폭 — 세션 밖(여백 컬럼)에서도 이 폭만 갖추면 모달을 세울 수 있다
export type DetailRound = {
  id: string;
  tier: string;
  ordinal: number;
  issueCount: number;
  document: { id: string; title: string; entity: { slug: string } };
};

export type ResultView = { kind: 'rejected' } | { kind: 'completed' };

export type RoundHeader = { title: string; tier: string };

export const findRound = (rounds: readonly ReviewRound[], workflowId: string): ReviewRound | null =>
  rounds.find((round) => round.workflow?.prismWorkflowId === workflowId) ?? null;

export const isRoundStale = (round: ReviewRound | null): boolean => round === null || round.state === 'RUNNING';

export type RecheckMode = 'idle' | 'settled' | 'stale';

export const recheckMode = (round: ReviewRound | null, answered: boolean, status: WorkflowStatus): RecheckMode => {
  if (!answered) {
    return 'idle';
  }

  if (round === null) {
    return 'stale';
  }

  if (!isRoundStale(round)) {
    return 'settled';
  }

  return status === 'completed' ? 'stale' : 'idle';
};

const documentTitle = (round: DetailRound) => `「${round.document.title || '제목 없음'}」`;

export const describeHeader = (round: DetailRound): RoundHeader => ({
  title: documentTitle(round),
  tier: TIER_OPTIONS.find((option) => option.tier === round.tier.toLowerCase())?.label ?? round.tier,
});

// 수치·문구의 조립은 결과 카드의 몫이다 — 여기는 결과가 설 수 있는가(완료·거절)만 가른다
export const describeResult = (round: ReviewRound): ResultView | null => {
  if (round.state !== 'COMPLETED') {
    return null;
  }

  return round.rejection ? { kind: 'rejected' } : { kind: 'completed' };
};
