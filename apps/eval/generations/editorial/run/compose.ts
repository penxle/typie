import { createFindRange } from '../../../core/text.ts';
import { leakLineLint } from '../checks.ts';
import { COMPOSE_REVIEW_SCHEMA, COMPOSE_SCHEMA, LOCAL_AXES } from '../contracts.ts';
import { GateLedger, normalizeReviewItems } from '../gates.ts';
import { renderComposeInputV2, renderEditorialComposeReviewInput, renderEditorialPlanBlock } from '../render.ts';
import { runAgentStage } from './agent-stage.ts';
import type { AnchorDraft } from '../../../core/contracts.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { AcceptedFinding, AcceptedStrength, EditorialPlan, ResolvedResearch } from '../types.ts';

export type ComposedFeedback = { category: string; layer: string; body: string; anchors: AnchorDraft[] };

export type EditorialReview = {
  characterization: string;
  strengths: { body: string; quoteStart: string; quoteEnd: string; matchStart: number | null; matchEnd: number | null }[];
  cleared: { axis: string; note: string }[];
  patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
  priority: { body: string; feedbackIndexes: number[] }[];
};

export type ComposeResult = {
  feedbacks: ComposedFeedback[];
  review: EditorialReview;
};

const FEEDBACKS_PATH = 'output/feedbacks.yaml';
const REPORT_PATH = 'output/report.yaml';

const COMPOSE_DELIVERABLE: Deliverable = {
  label: '피드백 문면',
  submitName: 'compose_feedbacks',
  submitDescription: '피드백 문면을 확정한다. 검사를 통과해야 접수된다.',
  outputs: {
    [FEEDBACKS_PATH]: { schema: COMPOSE_SCHEMA, lints: [leakLineLint], description: '피드백 문면 — 작가에게 전달되는 지적 본문' },
  },
};

const REVIEW_DELIVERABLE: Deliverable = {
  label: '작품 총평',
  submitName: 'report_review',
  submitDescription: '작품 총평을 확정한다. 검사를 통과해야 접수된다.',
  outputs: {
    [REPORT_PATH]: {
      schema: COMPOSE_REVIEW_SCHEMA,
      lints: [leakLineLint],
      description: '작품 총평 — 성격 규정·강점·무혐의·패턴·우선순위',
    },
  },
};

type ComposeRaw = { feedbacks: { findingIndexes: number[]; body: string }[] };

type ReviewRaw = {
  characterization: string;
  strengths: { body: string; quoteStart: string; quoteEnd: string }[];
  cleared: { axis: string; note: string }[];
  patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
  priority: { body: string; feedbackIndexes: number[] }[];
};

// 문면 확정. category는 축에서 파생한다 — 지적→축→계획의 추적선.
export const runCompose = async (
  ctx: RunContext,
  clients: LlmClients,
  workspace: Workspace,
  manuscriptFile: string,
  charter: string,
  plan: EditorialPlan,
  research: ResolvedResearch,
  findings: AcceptedFinding[],
  strengths: AcceptedStrength[],
  discardedByAxis: Map<string, number>,
): Promise<ComposeResult> => {
  const findRange = createFindRange(ctx.document.content);

  // 원고 순서로 정렬해 넘긴다 — 작성자가 좌표 없이 순서를 유지하면 된다.
  const ordered = findings.toSorted((a, b) => a.matchStart - b.matchStart);

  // 같은 대목을 다른 층위(작품 검토↔문면 교열)가 짚은 쌍 — 문면 상호 참조의 재료.
  const layerOf = (axis: string) => ((LOCAL_AXES as readonly string[]).includes(axis) ? 'local' : 'plan');
  const crossRefs = new Map<number, number[]>();
  for (const [i, a] of ordered.entries()) {
    const refs = ordered
      .map((b, j) => ({ b, j }))
      .filter(({ b, j }) => j !== i && layerOf(a.axis) !== layerOf(b.axis) && a.matchStart < b.matchEnd && b.matchStart < a.matchEnd)
      .map(({ j }) => j);
    if (refs.length > 0) crossRefs.set(i, refs);
  }

  const composed = await runAgentStage<ComposeRaw, { delivered: ComposedFeedback[] }>(ctx, {
    clients,
    stage: 'compose',
    ledgerKey: 'compose',
    prompt: ctx.prompts.compose,
    workspace,
    // 문면 작성은 전달받은 지적·강점이 재료의 전부다 — 원고 재열람은 설계상 없다.
    manuscriptAccess: false,
    system: [charter, '', renderEditorialPlanBlock(plan)].join('\n'),
    initial: [
      renderComposeInputV2(ordered, crossRefs),
      '',
      `지적들을 ${FEEDBACKS_PATH}의 피드백 문면으로 옮기고 compose_feedbacks로 확정하세요.`,
    ].join('\n'),
    search: null,
    deliverable: COMPOSE_DELIVERABLE,
    manuscriptFile,
    baseTools: [],
    onSubmit: (_path, value, { turn, ledger }) => {
      const seen = new Set<number>();
      const delivered: ComposedFeedback[] = [];
      for (const f of value.feedbacks) {
        const valid = f.findingIndexes.filter((i) => Number.isSafeInteger(i) && i >= 0 && i < ordered.length && !seen.has(i));
        for (const i of valid) seen.add(i);
        if (valid.length === 0) {
          ledger.events.push({ turn, kind: 'compose-empty-feedback', detail: f.body.slice(0, 40) });
          continue;
        }
        // 병합은 같은 축끼리만 — 위반은 다수 축을 남기고 나머지를 유실 처리한다.
        const byAxis = new Map<string, number[]>();
        for (const i of valid) {
          const axis = ordered[i].axis;
          byAxis.set(axis, [...(byAxis.get(axis) ?? []), i]);
        }
        const [majorityAxis, kept] = [...byAxis].toSorted((a, b) => b[1].length - a[1].length)[0];
        for (const [axis, indexes] of byAxis) {
          if (axis === majorityAxis) continue;
          for (const i of indexes) seen.delete(i);
          ledger.events.push({ turn, kind: 'compose-mixed-axis', detail: `${axis}: ${indexes.join(',')}` });
        }
        delivered.push({
          category: majorityAxis,
          layer: (LOCAL_AXES as readonly string[]).includes(majorityAxis) ? 'local' : 'plan',
          body: f.body,
          anchors: kept.map((i) => ({
            quoteStart: ordered[i].quoteStart,
            quoteEnd: ordered[i].quoteEnd,
            matchStart: ordered[i].matchStart,
            matchEnd: ordered[i].matchEnd,
          })),
        });
      }
      const missing = ordered.map((_, i) => i).filter((i) => !seen.has(i));
      if (missing.length > 0) ledger.events.push({ turn, kind: 'compose-missing', detail: missing.join(',') });
      return { accept: { delivered }, message: `접수. 피드백 ${delivered.length}건 확정.` };
    },
  });

  await ctx.phase('composeReview');

  // cleared의 근거는 계획 문면이다 — 지적 0건 축까지 검토 관점 블록으로 전달한다.
  const axisCounts = new Map(plan.axes.map((a) => [a.label, findings.filter((f) => f.axis === a.label).length]));

  const reviewed = await runAgentStage<ReviewRaw, EditorialReview>(ctx, {
    clients,
    stage: 'compose-review',
    ledgerKey: 'composeReview',
    prompt: ctx.prompts.composeReview,
    workspace,
    manuscriptAccess: false,
    system: '',
    initial: [
      renderEditorialComposeReviewInput(
        research,
        plan.axes.map((a) => ({
          label: a.label,
          inquiry: a.inquiry,
          findingCount: axisCounts.get(a.label) ?? 0,
          discardedCount: discardedByAxis.get(a.label) ?? 0,
        })),
        composed.value.delivered.map((f) => ({ category: f.category, body: f.body, anchorCount: f.anchors.length })),
        strengths,
      ),
      '',
      `작품 총평을 ${REPORT_PATH}에 작성하고 report_review로 확정하세요.`,
    ].join('\n'),
    search: null,
    deliverable: REVIEW_DELIVERABLE,
    manuscriptFile,
    baseTools: [],
    onSubmit: (_path, raw) => {
      const offered = new Map(strengths.map((s) => [`${s.quoteStart}|${s.quoteEnd}`, s]));
      const resolvedStrengths = (raw.strengths ?? []).map((s) => {
        const hit = offered.get(`${s.quoteStart}|${s.quoteEnd}`);
        if (hit) return { ...s, matchStart: hit.matchStart, matchEnd: hit.matchEnd };
        const range = findRange(s.quoteStart, s.quoteEnd, 0);
        return { ...s, matchStart: range?.rangeStart ?? null, matchEnd: range?.rangeEnd ?? null };
      });

      const gateLedger = new GateLedger();
      // cleared는 계획에 실재하고 지적이 0건인 축만 통과한다 — 그 외는 지어낸 무혐의다.
      // 제출이 유실된 축도 제외한다: 발견이 있었는데 게이트에 사라진 것은 무혐의가 아니다.
      const cleared = (raw.cleared ?? []).filter((c) => {
        const count = axisCounts.get(c.axis);
        if (count === undefined) return gateLedger.reject('review-cleared-unknown-axis', c.axis);
        if (count > 0) return gateLedger.reject('review-cleared-has-findings', c.axis);
        if ((discardedByAxis.get(c.axis) ?? 0) > 0) return gateLedger.reject('review-cleared-discarded-axis', c.axis);
        return c.note.trim().length > 0;
      });

      return {
        accept: {
          characterization: raw.characterization ?? '',
          strengths: resolvedStrengths,
          cleared,
          patterns: normalizeReviewItems(raw.patterns ?? [], composed.value.delivered.length, gateLedger),
          priority: normalizeReviewItems(raw.priority ?? [], composed.value.delivered.length, gateLedger),
        },
        message: '접수.',
      };
    },
  });

  return { feedbacks: composed.value.delivered, review: reviewed.value };
};
