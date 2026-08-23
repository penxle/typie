import { TIER_OPTIONS } from './tiers.ts';
import type { DataOf } from '@mearie/svelte';
import type { DashboardLayout_PrismReviewPassage_Query } from '$mearie';
import type { WorkflowStatus } from '../lib/conversation.ts';

export type ReviewRound = DataOf<DashboardLayout_PrismReviewPassage_Query>['prismSession']['reviewRounds'][number];

export type ResultView = { kind: 'rejected' } | { kind: 'summary'; counts: string; issues: string } | { kind: 'issues'; issues: string };

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

const documentTitle = (round: ReviewRound) => `「${round.document.title || '제목 없음'}」`;

export const describeHeader = (round: ReviewRound): RoundHeader => ({
  title: documentTitle(round),
  tier: TIER_OPTIONS.find((option) => option.tier === round.tier.toLowerCase())?.label ?? round.tier,
});

export const describeResult = (round: ReviewRound): ResultView | null => {
  if (round.state !== 'COMPLETED') {
    return null;
  }

  if (round.rejection) {
    return { kind: 'rejected' };
  }

  const issues = `본문 여백에 피드백 ${round.issueCount}개를 남겼어요`;

  if (round.conclusion) {
    const { patternsCount, prioritiesCount, strengthsCount } = round.conclusion;

    return {
      kind: 'summary',
      counts: `패턴 ${patternsCount} · 우선순위 ${prioritiesCount}${strengthsCount > 0 ? ` · 강점 ${strengthsCount}` : ''}`,
      issues,
    };
  }

  return { kind: 'issues', issues };
};
