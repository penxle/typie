// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { anchorQuote } from '@typie/prism';
import { z } from 'zod';
import type { StableSelection } from '@typie/editor-ffi/server';
import type { PrismReviewRoundState, PrismReviewThreadState, PrismReviewTier, PrismWorkflowState } from '@typie/lib/enums';
import type {
  Anchor,
  ConclusionAnchors,
  PrismReviewTierName,
  ResolvedAnchor,
  ReviewIssue,
  ReviewOutcome,
  ReviewPreviousContext,
  ReviewSeedMapping,
  ReviewThreadDisposition,
} from '@typie/prism';
import type { Dayjs } from 'dayjs';

export const ENUM_TO_TIER: Record<PrismReviewTier, PrismReviewTierName> = { LOW: 'low', MEDIUM: 'medium', HIGH: 'high' };

export const ConfirmInputSchema = z.union([
  z.object({
    decision: z.literal('confirmed'),
    versionId: z.string(),
    tier: z.enum(['LOW', 'MEDIUM', 'HIGH']),
    lineageId: z.string().optional(),
  }),
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

  // 총평이 없는 티어(low)의 결과는 지적 수만 낸다 — 총평 유무는 kind가 아니라 절의 유무가 가른다(prism result.ts).
  const c = outcome.conclusion;
  if (c === undefined) return { rejection: null, conclusion: null, issueCount: outcome.issues.length };

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
  strengths: { quote: string; body: string | null; anchors: ResolvedAnchor[] }[];
  verdicts: { trait: string; note: string | null }[];
  elevations: { trait: string; quote: string | null; body: string }[];
  patterns: { theme: string | null; body: string; issues: IssueBrief[] }[];
  priorities: { body: string; issues: IssueBrief[] }[];
};

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

const unresolved = (anchor: Anchor): ResolvedAnchor => ({ head: anchor.head, tail: anchor.tail, selection: null, text: null });

export type AnchorSite = { kind: 'issue' | 'strength' | 'elevation'; item: number; at: number; anchor: Anchor };
export type AnchorHit = { selection: StableSelection; text: string };
export type OutcomeAnchors = { issues: ResolvedAnchor[][]; conclusion: ConclusionAnchors };

// 해석·캡처는 결과 전체를 한 번에 한다 — 사이트 목록이 그 입력이고, 재조립이 결과와 평행한 배열로 되돌린다
export const outcomeAnchorSites = (outcome: ReviewOutcome | null): AnchorSite[] => {
  if (outcome === null || outcome.kind === 'rejected') return [];

  const sites: AnchorSite[] = outcome.issues.flatMap((issue, item) =>
    issue.anchors.map((anchor, at) => ({ kind: 'issue' as const, item, at, anchor })),
  );

  sites.push(
    ...(outcome.conclusion?.strengths ?? []).flatMap((strength, item) =>
      strength.anchors.map((anchor, at) => ({ kind: 'strength' as const, item, at, anchor })),
    ),
    ...(outcome.elevations ?? []).flatMap((elevation, item) =>
      elevation.anchors.map((anchor, at) => ({ kind: 'elevation' as const, item, at, anchor })),
    ),
  );
  return sites;
};

export const assembleOutcomeAnchors = (
  outcome: ReviewOutcome | null,
  sites: readonly AnchorSite[],
  hits: readonly (AnchorHit | null)[],
): OutcomeAnchors => {
  const reviewed = outcome !== null && outcome.kind !== 'rejected' ? outcome : null;
  const issues = reviewed === null ? [] : reviewed.issues.map((issue) => issue.anchors.map(unresolved));
  const strengths = (reviewed?.conclusion?.strengths ?? []).map((strength) => strength.anchors.map(unresolved));
  const elevations = (reviewed?.elevations ?? []).map((elevation) => elevation.anchors.map(unresolved));
  const buckets = { issue: issues, strength: strengths, elevation: elevations };

  for (const [index, site] of sites.entries()) {
    const hit = hits[index] ?? null;
    if (hit === null) continue;
    buckets[site.kind][site.item][site.at] = { head: site.anchor.head, tail: site.anchor.tail, selection: hit.selection, text: hit.text };
  }

  return { issues, conclusion: { strengths, elevations } };
};

export const unresolvedOutcomeAnchors = (outcome: ReviewOutcome | null): OutcomeAnchors => assembleOutcomeAnchors(outcome, [], []);

export const detailOutcome = (outcome: ReviewOutcome | null, conclusionAnchors: ConclusionAnchors | null): OutcomeDetail | null => {
  if (outcome === null || outcome.kind === 'rejected') return null;

  const { conclusion, issues } = outcome;
  if (conclusion === undefined) return null;

  return {
    understanding: conclusion.understanding,
    progress: conclusion.progress ?? null,
    strengths: (conclusion.strengths ?? []).map((strength, index) => {
      const anchors = conclusionAnchors?.strengths[index] ?? strength.anchors.map(unresolved);
      return { quote: anchorQuote(anchors), body: strength.body, anchors };
    }),
    verdicts: (outcome.verdicts ?? []).map((verdict) => ({ trait: verdict.trait, note: verdict.note })),
    elevations: (outcome.elevations ?? []).map((elevation, index) => ({
      trait: elevation.trait,
      quote: anchorQuote(conclusionAnchors?.elevations[index] ?? elevation.anchors.map(unresolved)) || null,
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
  if (outcome === null || outcome.kind === 'rejected') return false;

  const { conclusion } = outcome;
  if (conclusion === undefined) return false;

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

export const seedsPrefix = (roundId: string): string => `seeds/${roundId}`;

export const confirmResult = (
  key: string,
  tier: PrismReviewTierName,
  document: { id: string; title: string | null; subtitle: string | null; path: string },
  followup?: { previous: ReviewPreviousContext; seeds: string },
) => ({ decision: 'confirmed', key, tier, document, ...(followup !== undefined && followup) }) as const;

export type PreviousThreadSource = {
  id: string;
  pass: 'JUDGMENT' | 'STYLISTIC';
  trait: string;
  body: string | null;
  state: PrismReviewThreadState;
  issueId: string | null;
  anchors: ResolvedAnchor[];
  comments: { author: 'USER' | 'AI'; body: string; createdAt: Date }[];
};

const PASS_NAME = { JUDGMENT: 'judgment', STYLISTIC: 'stylistic' } as const;
const STATE_NAME = { OPEN: 'open', CLOSED: 'closed', RESOLVED: 'resolved', WITHDRAWN: 'withdrawn' } as const;

// AI 코멘트는 지난 리뷰 자신의 산출물이라 싣지 않는다. fresh = base 회차가 입력을 굳힌 뒤 달린 답글 — prism은 시각을 모르므로 여기서 판별해 표지만 넘긴다.
export const buildPreviousContext = (input: {
  base: { title: string | null; subtitle: string | null; versionId: string; createdAt: Date };
  threads: PreviousThreadSource[];
}): ReviewPreviousContext => ({
  title: input.base.title,
  subtitle: input.base.subtitle,
  path: manuscriptPath(input.base.versionId),
  threads: input.threads.map((thread) => ({
    id: thread.id,
    pass: PASS_NAME[thread.pass],
    trait: thread.trait,
    body: thread.body ?? '',
    anchors: thread.anchors.map(({ head, tail }) => ({ head, tail })),
    replies: thread.comments
      .filter((comment) => comment.author === 'USER')
      .toSorted((a, b) => a.createdAt.getTime() - b.createdAt.getTime())
      .map((comment) => ({ body: comment.body, fresh: comment.createdAt.getTime() > input.base.createdAt.getTime() })),
    state: STATE_NAME[thread.state],
    ...(thread.issueId !== null && thread.issueId !== '' && { issue: thread.issueId }),
  })),
});

export const seedUploads = (
  roundId: string,
  seeds: readonly ReviewSeedMapping[],
  contents: ReadonlyMap<string, string | null>,
): { path: string; content: string }[] | { missing: string } => {
  const files: { path: string; content: string }[] = [];

  for (const seed of seeds) {
    const content = contents.get(seed.from) ?? null;
    if (content === null) return { missing: seed.from };
    files.push({ path: `${seedsPrefix(roundId)}/${seed.to}`, content });
  }

  return files;
};

export const pickVersion = (latest: (Snapshot & { version: number }) | null, snap: Snapshot): { reuse: boolean; version: number } => {
  if (latest && latest.content === snap.content && latest.title === snap.title && latest.subtitle === snap.subtitle)
    return { reuse: true, version: latest.version };
  return { reuse: false, version: (latest?.version ?? 0) + 1 };
};

export const roundState = (round: { closedAt: Dayjs | null }, workflow: { state: PrismWorkflowState } | null): PrismReviewRoundState => {
  if (workflow !== null) return workflow.state;
  return round.closedAt === null ? 'PENDING' : 'CANCELED';
};

export const lineageLocked = (rounds: readonly { closedAt: Dayjs | null; workflowState: PrismWorkflowState | null }[]): boolean =>
  rounds.some((round) => {
    const state = roundState(round, round.workflowState === null ? null : { state: round.workflowState });
    return state === 'PENDING' || state === 'RUNNING';
  });

export type ProjectedThread = {
  issueIndex: number;
  issueId: string | null;
  trait: string;
  pass: 'JUDGMENT' | 'STYLISTIC';
  body: string | null;
};

export type CarriedSeat = { threadId: string; issueIndex: number };
export type ProjectionPlan = { fresh: ProjectedThread[]; carried: CarriedSeat[]; dispositions: ReviewThreadDisposition[] };

// thread 표지 없는 이슈만 새 스레드가 된다 — 표지 이슈는 지난 스레드의 계속이라 좌석만 늘린다
// 좌석 앵커는 여기 실리지 않는다 — 사영이 OutcomeAnchors.issues[issueIndex]에서 가져온다(출처는 하나)
export const planProjection = (outcome: ReviewOutcome | null): ProjectionPlan => {
  if (outcome === null || outcome.kind === 'rejected') return { fresh: [], carried: [], dispositions: [] };

  const fresh: ProjectedThread[] = [];
  const carried: CarriedSeat[] = [];

  for (const [index, issue] of outcome.issues.entries()) {
    if (issue.thread === undefined) {
      fresh.push({
        issueIndex: index,
        issueId: issue.id ?? null,
        trait: issue.trait,
        pass: issue.pass === 'judgment' ? 'JUDGMENT' : 'STYLISTIC',
        body: issue.body,
      });
    } else {
      carried.push({ threadId: issue.thread, issueIndex: index });
    }
  }

  return { fresh, carried, dispositions: outcome.dispositions ?? [] };
};

export const dispositionSummary = (
  outcome: ReviewOutcome | null,
): { carried: number; resolved: number; withdrawn: number; new: number } | null => {
  if (outcome === null || outcome.kind === 'rejected' || outcome.dispositions === undefined) return null;

  const plan = planProjection(outcome);

  return {
    carried: plan.carried.length,
    resolved: plan.dispositions.filter((disposition) => disposition.verdict === 'resolved').length,
    withdrawn: plan.dispositions.filter((disposition) => disposition.verdict === 'withdrawn').length,
    new: plan.fresh.length,
  };
};

// 계보에 회차가 둘 이상 있어야 '신규'가 성립한다 — 첫 회차의 스레드는 전부 처음이라 표지가 의미 없다
export const threadIsNew = (bornRoundId: string, viewRoundId: string, completedRoundsInLineage: number): boolean =>
  bornRoundId === viewRoundId && completedRoundsInLineage >= 2;

export const aiCommentId = (threadId: string, roundId: string): string => `${threadId}.${roundId}`;
