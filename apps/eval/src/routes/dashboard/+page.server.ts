import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import {
  anchorMatchRate,
  categoryComplianceRate,
  cohenKappa,
  deriveWinRates,
  falsePositiveRate,
  feedbackCountDistribution,
  pairwiseFromRanking,
  sanityPassRate,
} from '$lib/domain/aggregate.ts';
import { createDb, Feedbacks, FeedbackSets, Judgments, Rounds, Runs, Tasks, Variants } from '$lib/server/db/index.ts';
import type { JudgmentResult, PairVerdict } from '$lib/domain/types.ts';
import type { PageServerLoad } from './$types';

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

    const v0 = variants.find((v) => v.label === 'V0');

    const rankingEntries = [];
    const overlapPairs: [PairVerdict, PairVerdict][] = [];
    const sanityVerdicts: PairVerdict[] = [];
    const pairTallies = new Map<string, { win: number; tie: number; loss: number }>();
    const fpEntries = [];

    for (const task of tasks) {
      const taskJudgments = confirmed.filter((j) => j.taskId === task.id);

      for (const [judgmentIndex, judgment] of taskJudgments.entries()) {
        const result = judgment.result as JudgmentResult;

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
        }

        if (task.kind === 'sanity' && result.kind === 'pair') {
          sanityVerdicts.push(result.verdict);
        } else if (task.kind === 'ranking' && result.kind === 'ranking' && v0 && judgmentIndex === 0) {
          const v0SetId = task.setIds.find((s) => setVariant.get(s) === v0.id);
          if (!v0SetId) continue;
          const candidateSetIds = new Map(
            task.setIds.filter((s) => s !== v0SetId).map((s) => [setVariant.get(s) ?? 'unknown', s] as const),
          );
          rankingEntries.push({ ranks: result.ranks, v0SetId, candidateSetIds });
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
        if (firstResult.kind !== 'ranking' || secondResult.kind !== 'ranking') continue;
        const candidateSetIds = task.setIds.filter((s) => s !== v0SetId);
        for (const setId of candidateSetIds) {
          overlapPairs.push([
            pairwiseFromRanking(firstResult.ranks, setId, v0SetId),
            pairwiseFromRanking(secondResult.ranks, setId, v0SetId),
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

    summaries.push({
      roundId: round.id,
      stage: round.stage,
      taskCount: tasks.length,
      confirmedCount: confirmed.length,
      categoryCompliance,
      variants: variantRows,
      kappa: cohenKappa(overlapPairs),
      sanityPass: sanityPassRate(sanityVerdicts),
    });
  }

  return { summaries };
};
