// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { z } from 'zod';
import type { PrismReviewRoundState, PrismReviewTier, PrismWorkflowState } from '@typie/lib/enums';
import type { PrismReviewTierName, ReviewOutcome } from '@typie/prism';
import type { Dayjs } from 'dayjs';

export const ENUM_TO_TIER: Record<PrismReviewTier, PrismReviewTierName> = { LOW: 'low', MEDIUM: 'medium', HIGH: 'high' };

export const ConfirmInputSchema = z.union([
  z.object({ decision: z.literal('confirmed'), documentId: z.string(), tier: z.enum(['LOW', 'MEDIUM', 'HIGH']) }),
  z.object({ decision: z.literal('declined') }),
]);

export type OutcomeSummary = {
  rejection: { message: string } | null;
  conclusion: {
    understanding: string | null;
    progress: string | null;
    strengthsCount: number;
    verdictsCount: number;
    elevationsCount: number;
    patternsCount: number;
    prioritiesCount: number;
  } | null;
  issueCount: number;
};

export const summarizeOutcome = (outcome: ReviewOutcome | null): OutcomeSummary => {
  if (outcome === null) return { rejection: null, conclusion: null, issueCount: 0 };
  if (outcome.kind === 'rejected') return { rejection: { message: outcome.rejected.message }, conclusion: null, issueCount: 0 };
  if (outcome.kind === 'issues') return { rejection: null, conclusion: null, issueCount: outcome.issues.length };

  const c = outcome.conclusion;
  return {
    rejection: null,
    conclusion: {
      understanding: c.understanding,
      progress: c.progress ?? null,
      strengthsCount: c.strengths?.length ?? 0,
      verdictsCount: outcome.verdicts?.length ?? 0,
      elevationsCount: outcome.elevations?.length ?? 0,
      patternsCount: c.patterns.length,
      prioritiesCount: c.priorities.length,
    },
    issueCount: outcome.issues.length,
  };
};

export type Snapshot = { title: string | null; subtitle: string | null; content: string };

export const manuscriptPath = (versionId: string): string => `manuscript/${versionId}.txt`;

export const confirmResult = (
  key: string,
  tier: PrismReviewTierName,
  document: { title: string | null; subtitle: string | null; path: string },
) => ({ decision: 'confirmed', key, tier, document }) as const;

export const pickVersion = (latest: (Snapshot & { version: number }) | null, snap: Snapshot): { reuse: boolean; version: number } => {
  if (latest && latest.content === snap.content && latest.title === snap.title && latest.subtitle === snap.subtitle)
    return { reuse: true, version: latest.version };
  return { reuse: false, version: (latest?.version ?? 0) + 1 };
};

export const roundState = (round: { closedAt: Dayjs | null }, workflow: { state: PrismWorkflowState } | null): PrismReviewRoundState => {
  if (workflow !== null) return workflow.state;
  return round.closedAt === null ? 'PENDING' : 'CANCELED';
};
