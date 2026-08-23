// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { anchorQuote } from '@typie/prism';
import { z } from 'zod';
import type { PrismReviewRoundState, PrismReviewTier, PrismWorkflowState } from '@typie/lib/enums';
import type { Anchor, PrismReviewTierName, ReviewIssue, ReviewOutcome } from '@typie/prism';
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

export type IssueBrief = { index: number; trait: string };

export type OutcomeDetail = {
  understanding: string | null;
  progress: string | null;
  strengths: { quote: string; body: string | null; anchor: Anchor }[];
  verdicts: { trait: string; label: string | null; note: string | null }[];
  elevations: { trait: string; quote: string | null; body: string }[];
  patterns: { theme: string | null; body: string; issues: IssueBrief[] }[];
  priorities: { body: string; issues: IssueBrief[] }[];
};

// 판정점의 작가 대면 번역 — 숫자는 성적표로 읽히므로 화면에 세우지 않는다.
const VERDICT_LABELS: Record<number, string> = { 1: '아직', 2: '흔들림', 3: '자리 잡음', 4: '단단함' };

// 참조는 이슈 id(현행)와 배열 번호(구 결과) 양쪽으로 온다 — 두 형태가 갈리는 자리가 여기뿐이라 여기서 해소한다.
const briefsOfRefs = (refs: readonly (number | string)[], issues: ReviewIssue[]): IssueBrief[] => {
  const seen = new Set<number>();

  return refs.flatMap((ref) => {
    const index = typeof ref === 'number' ? ref : issues.findIndex((issue) => issue.id === ref);
    const issue = issues[index];
    if (issue === undefined || seen.has(index)) return [];
    seen.add(index);
    return [{ index, trait: issue.trait }];
  });
};

// 강점은 인용과 설명뿐이라 인용을 원문에서 못 찾으면 항목이 빈다 — 앵커가 들고 있는 머리·꼬리로 세운다.
const strengthQuote = (content: string, strength: Anchor): string => {
  const quote = anchorQuote(content, [strength]);
  if (quote.length > 0) return quote;
  return strength.head === strength.tail ? strength.head : `${strength.head} ⋯ ${strength.tail}`;
};

export const detailOutcome = (outcome: ReviewOutcome | null, content: string): OutcomeDetail | null => {
  if (outcome === null || outcome.kind !== 'feedback') return null;

  const { conclusion, issues } = outcome;

  return {
    understanding: conclusion.understanding,
    progress: conclusion.progress ?? null,
    strengths: (conclusion.strengths ?? []).map((strength) => ({
      quote: strengthQuote(content, strength),
      body: strength.body,
      anchor: { start: strength.start, end: strength.end, head: strength.head, tail: strength.tail },
    })),
    verdicts: (outcome.verdicts ?? []).map((verdict) => ({
      trait: verdict.trait,
      label: VERDICT_LABELS[verdict.point] ?? null,
      note: verdict.note,
    })),
    elevations: (outcome.elevations ?? []).map((elevation) => ({
      trait: elevation.trait,
      quote: anchorQuote(content, elevation.anchors) || null,
      body: elevation.body,
    })),
    patterns: conclusion.patterns.map((pattern) => ({
      theme: pattern.theme,
      body: pattern.body,
      issues: briefsOfRefs(pattern.issues, issues),
    })),
    priorities: conclusion.priorities.map((priority) => ({ body: priority.body, issues: briefsOfRefs(priority.issues, issues) })),
  };
};

export const hasDetail = (outcome: ReviewOutcome | null): boolean => {
  if (outcome === null || outcome.kind !== 'feedback') return false;

  const { conclusion } = outcome;

  return (
    (conclusion.understanding ?? '').trim().length > 0 ||
    (conclusion.progress ?? '').trim().length > 0 ||
    (conclusion.strengths?.length ?? 0) > 0 ||
    (outcome.verdicts?.length ?? 0) > 0 ||
    (outcome.elevations?.length ?? 0) > 0 ||
    conclusion.patterns.length > 0 ||
    conclusion.priorities.length > 0
  );
};

export type Snapshot = { title: string | null; subtitle: string | null; content: string };

export const manuscriptPath = (versionId: string): string => `manuscript/${versionId}.txt`;

export const confirmResult = (
  key: string,
  tier: PrismReviewTierName,
  document: { id: string; title: string | null; subtitle: string | null; path: string },
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

export type ProjectedThread = {
  issueIndex: number;
  issueId: string | null;
  trait: string;
  pass: 'JUDGMENT' | 'STYLISTIC';
  body: string | null;
  anchors: Anchor[];
};

export const threadsFromResult = (outcome: ReviewOutcome | null): ProjectedThread[] => {
  if (outcome === null || outcome.kind === 'rejected') return [];

  return outcome.issues.map((issue, index) => ({
    issueIndex: index,
    issueId: issue.id ?? null,
    trait: issue.trait,
    pass: issue.pass === 'judgment' ? 'JUDGMENT' : 'STYLISTIC',
    body: issue.body,
    anchors: issue.anchors,
  }));
};

// 인용은 저장하지 않는다 — 리뷰 시점 판본에서 조회 때마다 자른다
export const threadQuote = (content: string, anchors: Anchor[]): string => anchorQuote(content, anchors);
