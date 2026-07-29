import Anthropic from '@anthropic-ai/sdk';
import { renderResearchCharter } from '../render.ts';
import { runCompose } from './compose.ts';
import { runExecute } from './execute.ts';
import { buildItems } from './items.ts';
import { runLocal } from './local.ts';
import { runPlan } from './plan.ts';
import { createSearch, runResearch } from './research.ts';
import type { GenerationRunner } from '../../../core/worker/run-contracts.ts';

export const editorialRunner: GenerationRunner = async (ctx) => {
  // 게이트웨이의 Anthropic 전용 엔드포인트. Unified Billing이라 Anthropic 키 없이 Cloudflare
  // 토큰만으로 통과한다 — x-api-key 대신 Authorization: Bearer로 나간다.
  const client = new Anthropic({
    apiKey: null,
    authToken: ctx.env.CLOUDFLARE_API_KEY,
    baseURL: ctx.env.CLOUDFLARE_AIGATEWAY_URL,
  });
  const search = createSearch(ctx);

  await ctx.phase('research');
  const research = await runResearch(ctx, client, search);
  const charter = renderResearchCharter(research.research);
  await ctx.ledger('research', research.research);
  await ctx.ledger('ledger/research', research.ledger);

  await ctx.phase('plan');
  const plan = await runPlan(ctx, client, charter, search, research.ledger.tools);
  await ctx.ledger('plan', { final: plan.plan, rounds: plan.rounds });
  await ctx.ledger('ledger/plan', plan.ledger);

  await ctx.phase('execute');
  const execute = await runExecute(ctx, client, charter, plan.plan, research.research);
  await ctx.ledger('execute', {
    findings: execute.findings,
    strengths: execute.strengths,
    discardedAxes: [...execute.discardedByAxis],
  });
  await ctx.ledger('ledger/execute', execute.ledger);

  await ctx.phase('local');
  const local = await runLocal(ctx, client, charter, research.research);
  await ctx.ledger('local', { findings: local.findings });
  await ctx.ledger('ledger/local', local.ledger);

  await ctx.phase('compose');
  const composed = await runCompose(
    ctx,
    client,
    charter,
    plan.plan,
    research.research,
    [...execute.findings, ...local.findings],
    execute.strengths,
    execute.discardedByAxis,
  );
  await ctx.ledger('ledger/compose', composed.events);

  return {
    items: buildItems({
      characterization: composed.review.characterization,
      feedbacks: composed.feedbacks,
      strengths: composed.review.strengths,
      cleared: composed.review.cleared,
      patterns: composed.review.patterns,
      priority: composed.review.priority,
    }),
  };
};
