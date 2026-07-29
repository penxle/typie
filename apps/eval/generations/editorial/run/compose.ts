import { createFindRange } from '../../../core/text.ts';
import { callTool, LLM_STEP } from '../../../core/worker/llm.ts';
import { EDITORIAL_COMPOSE_REVIEW_TOOL, EDITORIAL_COMPOSE_TOOL, LOCAL_AXES } from '../contracts.ts';
import { GateLedger, normalizeReviewItems } from '../gates.ts';
import { renderComposeInputV2, renderEditorialComposeReviewInput, renderEditorialPlanBlock } from '../render.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { AnchorDraft } from '../../../core/contracts.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
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
  events: { kind: string; detail: string }[];
};

// 문면 확정. category는 축에서 파생한다 — 지적→축→계획의 추적선.
export const runCompose = async (
  ctx: RunContext,
  client: Anthropic,
  charter: string,
  plan: EditorialPlan,
  research: ResolvedResearch,
  findings: AcceptedFinding[],
  strengths: AcceptedStrength[],
  discardedByAxis: Map<string, number>,
): Promise<ComposeResult> => {
  const findRange = createFindRange(ctx.document.content);
  const composeOptions = { conventions: [charter, '', renderEditorialPlanBlock(plan)].join('\n'), cache: false };

  // 원고 순서로 정렬해 넘긴다 — 작성자가 좌표 없이 순서를 유지하면 된다.
  const ordered = findings.toSorted((a, b) => a.matchStart - b.matchStart);

  const composed = (await ctx.step.do('compose', LLM_STEP, async () => {
    const { value } = await ctx.cached('compose', async (usage) => {
      type Raw = { feedbacks: { findingIndexes: number[]; body: string }[] };
      const raw = await callTool<Raw>(
        client,
        ctx.prompts.compose,
        EDITORIAL_COMPOSE_TOOL,
        renderComposeInputV2(ordered),
        usage,
        composeOptions,
      );

      const events: { kind: string; detail: string }[] = [];
      const seen = new Set<number>();
      const delivered: ComposedFeedback[] = [];
      for (const f of raw.feedbacks) {
        const valid = f.findingIndexes.filter((i) => Number.isSafeInteger(i) && i >= 0 && i < ordered.length && !seen.has(i));
        for (const i of valid) seen.add(i);
        if (valid.length === 0) {
          events.push({ kind: 'compose-empty-feedback', detail: f.body.slice(0, 40) });
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
          events.push({ kind: 'compose-mixed-axis', detail: `${axis}: ${indexes.join(',')}` });
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
      if (missing.length > 0) events.push({ kind: 'compose-missing', detail: missing.join(',') });
      return { delivered, events };
    });
    return value as never;
  })) as unknown as { delivered: ComposedFeedback[]; events: { kind: string; detail: string }[] };

  await ctx.phase('composeReview');

  const review = (await ctx.step.do('compose-review', LLM_STEP, async () => {
    const { value } = await ctx.cached('compose-review', async (usage) => {
      type Raw = {
        characterization: string;
        strengths: { body: string; quoteStart: string; quoteEnd: string }[];
        cleared: { axis: string; note: string }[];
        patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
        priority: { body: string; feedbackIndexes: number[] }[];
      };
      // cleared의 근거는 계획 문면이다 — 지적 0건 축까지 검토 관점 블록으로 전달한다.
      const axisCounts = new Map(plan.axes.map((a) => [a.label, findings.filter((f) => f.axis === a.label).length]));
      const raw = await callTool<Raw>(
        client,
        ctx.prompts.composeReview,
        EDITORIAL_COMPOSE_REVIEW_TOOL,
        renderEditorialComposeReviewInput(
          research,
          plan.axes.map((a) => ({
            label: a.label,
            inquiry: a.inquiry,
            findingCount: axisCounts.get(a.label) ?? 0,
            discardedCount: discardedByAxis.get(a.label) ?? 0,
          })),
          composed.delivered.map((f) => ({ category: f.category, body: f.body, anchorCount: f.anchors.length })),
          strengths,
        ),
        usage,
      );

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
        characterization: raw.characterization ?? '',
        strengths: resolvedStrengths,
        cleared,
        patterns: normalizeReviewItems(raw.patterns ?? [], composed.delivered.length, gateLedger),
        priority: normalizeReviewItems(raw.priority ?? [], composed.delivered.length, gateLedger),
      };
    });
    return value as never;
  })) as unknown as EditorialReview;

  return { feedbacks: composed.delivered, review, events: composed.events };
};
