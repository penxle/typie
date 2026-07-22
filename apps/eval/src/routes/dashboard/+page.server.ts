import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import {
  anchorMatchRate,
  averageScores,
  categoryComplianceRate,
  cohenKappa,
  deriveWinRates,
  falsePositiveRate,
  feedbackCountDistribution,
  pairwiseFromRanking,
  ranksFromScores,
  sanityPassRate,
} from '$lib/domain/aggregate.ts';
import { FEEDBACK_LABELS } from '$lib/domain/feedback-labels.ts';
import { createDb, Feedbacks, FeedbackSets, Judgments, Rounds, Runs, Tasks, Variants } from '$lib/server/db/index.ts';
import type { JudgmentResult, PairVerdict } from '$lib/domain/types.ts';
import type { PageServerLoad } from './$types';

const labelByKey = new Map(FEEDBACK_LABELS.map((label) => [label.key, label]));

// 기준선 variant 라벨 — 'V0'은 테스트 라운드 시절 라벨, '현행'은 라운드 1부터의 라벨.
const BASELINE_LABELS = new Set(['V0', '현행']);

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const rounds = await db.select().from(Rounds);
  const variants = await db.select().from(Variants);
  const variantLabels = new Map(variants.map((v) => [v.id, v.label]));

  const summaries = [];
  for (const round of rounds) {
    const tasks = await db.select().from(Tasks).where(eq(Tasks.roundId, round.id));
    const taskIds = tasks.map((t) => t.id);
    const judgments = taskIds.length > 0 ? await db.select().from(Judgments).where(inArray(Judgments.taskId, taskIds)) : [];
    const confirmed = judgments.filter((j) => !j.draft && j.result);

    const allSetIds = [...new Set(tasks.flatMap((t) => t.setIds))];
    const sets = allSetIds.length > 0 ? await db.select().from(FeedbackSets).where(inArray(FeedbackSets.id, allSetIds)) : [];
    const setVariant = new Map(sets.map((s) => [s.id, s.variantId]));
    const feedbacks = allSetIds.length > 0 ? await db.select().from(Feedbacks).where(inArray(Feedbacks.setId, allSetIds)) : [];
    const feedbackSetId = new Map(feedbacks.map((f) => [f.id, f.setId]));

    const v0 = variants.find((v) => BASELINE_LABELS.has(v.label));

    const rankingEntries = [];
    const overlapPairs: [PairVerdict, PairVerdict][] = [];
    const sanityVerdicts: PairVerdict[] = [];
    const pairTallies = new Map<string, { win: number; tie: number; loss: number }>();
    const fpEntries = [];
    const scoreEntries: { variantId: string; score: number }[] = [];
    const labelDist = new Map<string, Map<string, number>>();
    const labelComments = new Map<string, { labelNames: string[]; comment: string }[]>();

    for (const task of tasks) {
      const taskJudgments = confirmed.filter((j) => j.taskId === task.id);

      for (const [judgmentIndex, judgment] of taskJudgments.entries()) {
        const result = judgment.result as JudgmentResult;
        const normalized = result.kind === 'scores' ? { kind: 'ranking' as const, ranks: ranksFromScores(result.scores) } : result;

        if (task.kind !== 'sanity') {
          for (const setId of task.setIds) {
            const variantId = setVariant.get(setId);
            if (!variantId) continue;
            const setFeedbacks = feedbacks.filter((f) => f.setId === setId);
            fpEntries.push({
              variantId,
              feedbackCount: setFeedbacks.length,
              flaggedCount: setFeedbacks.filter((f) => judgment.falsePositiveFeedbackIds.includes(f.id)).length,
            });
          }

          if (result.kind === 'scores') {
            for (const { setId, score } of result.scores) {
              const variantId = setVariant.get(setId);
              if (!variantId) continue;
              scoreEntries.push({ variantId, score });
            }
          }

          for (const [feedbackId, entry] of Object.entries(judgment.feedbackLabels ?? {})) {
            const setId = feedbackSetId.get(feedbackId);
            const variantId = setId ? setVariant.get(setId) : undefined;
            if (!variantId) continue;

            const dist = labelDist.get(variantId) ?? new Map<string, number>();
            for (const labelKey of entry.labels) {
              dist.set(labelKey, (dist.get(labelKey) ?? 0) + 1);
            }
            labelDist.set(variantId, dist);

            if (entry.comment) {
              const labelNames = entry.labels.map((key) => labelByKey.get(key)?.name ?? key);
              const comments = labelComments.get(variantId) ?? [];
              comments.push({ labelNames, comment: entry.comment });
              labelComments.set(variantId, comments);
            }
          }
        }

        if (task.kind === 'sanity' && result.kind === 'pair') {
          sanityVerdicts.push(result.verdict);
        } else if (task.kind === 'ranking' && normalized.kind === 'ranking' && v0 && judgmentIndex === 0) {
          const v0SetId = task.setIds.find((s) => setVariant.get(s) === v0.id);
          if (!v0SetId) continue;
          const candidateSetIds = new Map(
            task.setIds.filter((s) => s !== v0SetId).map((s) => [setVariant.get(s) ?? 'unknown', s] as const),
          );
          rankingEntries.push({ ranks: normalized.ranks, v0SetId, candidateSetIds });
        } else if (task.kind === 'pair' && result.kind === 'pair' && v0) {
          const [aSetId, bSetId] = task.setIds;
          const aVariant = setVariant.get(aSetId);
          const candidateVariant = aVariant === v0.id ? setVariant.get(bSetId) : aVariant;
          if (!candidateVariant) continue;
          const tally = pairTallies.get(candidateVariant) ?? { win: 0, tie: 0, loss: 0 };
          const candidateIsA = aVariant !== v0.id;
          if (result.verdict === 'tie') tally.tie++;
          else if ((result.verdict === 'a') === candidateIsA) tally.win++;
          else tally.loss++;
          pairTallies.set(candidateVariant, tally);
        }
      }

      if (task.kind === 'ranking' && task.requiredJudgments === 2 && taskJudgments.length >= 2 && v0) {
        const v0SetId = task.setIds.find((s) => setVariant.get(s) === v0.id);
        if (!v0SetId) continue;
        const [first, second] = taskJudgments;
        const firstResult = first.result as JudgmentResult;
        const secondResult = second.result as JudgmentResult;
        const firstNormalized =
          firstResult.kind === 'scores' ? { kind: 'ranking' as const, ranks: ranksFromScores(firstResult.scores) } : firstResult;
        const secondNormalized =
          secondResult.kind === 'scores' ? { kind: 'ranking' as const, ranks: ranksFromScores(secondResult.scores) } : secondResult;
        if (firstNormalized.kind !== 'ranking' || secondNormalized.kind !== 'ranking') continue;
        const candidateSetIds = task.setIds.filter((s) => s !== v0SetId);
        for (const setId of candidateSetIds) {
          overlapPairs.push([
            pairwiseFromRanking(firstNormalized.ranks, setId, v0SetId),
            pairwiseFromRanking(secondNormalized.ranks, setId, v0SetId),
          ]);
        }
      }
    }

    const winRates = deriveWinRates(rankingEntries);
    for (const [variantId, tally] of pairTallies) {
      const existing = winRates.get(variantId) ?? { win: 0, tie: 0, loss: 0 };
      winRates.set(variantId, {
        win: existing.win + tally.win,
        tie: existing.tie + tally.tie,
        loss: existing.loss + tally.loss,
      });
    }

    const nonSanitySetIds = [...new Set(tasks.filter((t) => t.kind !== 'sanity').flatMap((t) => t.setIds))];
    const anchorEntries = [];
    const countEntries = [];
    const roundCategories: (string | null)[] = [];
    for (const setId of nonSanitySetIds) {
      const variantId = setVariant.get(setId);
      if (!variantId) continue;
      const setFeedbacks = feedbacks.filter((f) => f.setId === setId);
      anchorEntries.push({
        variantId,
        matchedCount: setFeedbacks.filter((f) => f.matchStart !== null).length,
        feedbackCount: setFeedbacks.length,
      });
      countEntries.push({ variantId, feedbackCount: setFeedbacks.length });
      roundCategories.push(...setFeedbacks.map((f) => f.category));
    }
    const anchorRates = anchorMatchRate(anchorEntries);
    const countDist = feedbackCountDistribution(countEntries);
    const categoryCompliance = categoryComplianceRate(roundCategories);

    const runIds = [...new Set(sets.map((s) => s.runId))];
    const runs = runIds.length > 0 ? await db.select().from(Runs).where(inArray(Runs.id, runIds)) : [];
    const tokensByVariant = new Map<string, number>();
    for (const run of runs) {
      const meta = run.meta as { usage?: { promptTokens?: number; completionTokens?: number } } | null;
      const tokens = (meta?.usage?.promptTokens ?? 0) + (meta?.usage?.completionTokens ?? 0);
      tokensByVariant.set(run.variantId, (tokensByVariant.get(run.variantId) ?? 0) + tokens);
    }

    const fpRates = falsePositiveRate(fpEntries);
    const variantRows = [...new Set(sets.map((s) => s.variantId))]
      .map((variantId) => {
        const tally = winRates.get(variantId) ?? { win: 0, tie: 0, loss: 0 };
        return {
          label: variantLabels.get(variantId) ?? variantId,
          isBaseline: variantId === v0?.id,
          win: tally.win,
          tie: tally.tie,
          loss: tally.loss,
          falsePositive: fpRates.get(variantId) ?? NaN,
          anchorMatch: anchorRates.get(variantId) ?? NaN,
          zeroCount: countDist.get(variantId)?.zero ?? 0,
          over10Count: countDist.get(variantId)?.over10 ?? 0,
          tokens: tokensByVariant.get(variantId) ?? 0,
        };
      })
      .toSorted((a, b) => Number(b.isBaseline) - Number(a.isBaseline));

    // 평가자별 기여 건수(내림차순) — 이메일은 평가자 화면에 노출하지 않고 익명 순번으로만 보여준다.
    const byEvaluator = new Map<string, number>();
    for (const judgment of confirmed) {
      byEvaluator.set(judgment.evaluatorEmail, (byEvaluator.get(judgment.evaluatorEmail) ?? 0) + 1);
    }
    const contributions = [...byEvaluator.values()].toSorted((a, b) => b - a);

    const labelDistByLabel: Record<string, Record<string, number>> = {};
    for (const [variantId, dist] of labelDist) {
      labelDistByLabel[variantLabels.get(variantId) ?? variantId] = Object.fromEntries(dist);
    }
    const labelCommentsByLabel: Record<string, { labelNames: string[]; comment: string }[]> = {};
    for (const [variantId, comments] of labelComments) {
      labelCommentsByLabel[variantLabels.get(variantId) ?? variantId] = comments;
    }

    const avgScores = averageScores(scoreEntries);
    const avgScoreByLabel: Record<string, number> = {};
    for (const [variantId, avg] of avgScores) {
      avgScoreByLabel[variantLabels.get(variantId) ?? variantId] = avg;
    }

    summaries.push({
      roundId: round.id,
      stage: round.stage,
      taskCount: tasks.length,
      requiredTotal: tasks.reduce((sum, t) => sum + (t.requiredJudgments ?? 1), 0),
      confirmedCount: confirmed.length,
      contributions,
      categoryCompliance,
      variants: variantRows,
      kappa: cohenKappa(overlapPairs),
      sanityPass: sanityPassRate(sanityVerdicts),
      labelDist: labelDistByLabel,
      labelComments: labelCommentsByLabel,
      avgScore: avgScoreByLabel,
    });
  }

  return { summaries };
};
