import Anthropic from '@anthropic-ai/sdk';
import OpenAI from 'openai';
import { Workspace } from '../../../core/worker/workspace.ts';
import { renderResearchCharter } from '../render.ts';
import { runCompose } from './compose.ts';
import { runExecute } from './execute.ts';
import { buildItems } from './items.ts';
import { runLocal } from './local.ts';
import { runPlan } from './plan.ts';
import { createSearch, runResearch } from './research.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { GenerationRunner } from '../../../core/worker/run-contracts.ts';

export const editorialRunner: GenerationRunner = async (ctx) => {
  // 경로 둘을 함께 연다. anthropic/ 모델은 게이트웨이의 Anthropic 전용 엔드포인트로(Unified
  // Billing이라 Anthropic 키 없이 Cloudflare 토큰만으로 통과 — x-api-key 대신 Authorization:
  // Bearer), 그 외 모델은 compat 엔드포인트로 나간다. 어느 쪽인지는 단계 프롬프트의 모델이 정한다.
  const clients: LlmClients = {
    anthropic: new Anthropic({
      apiKey: null,
      authToken: ctx.env.CLOUDFLARE_API_KEY,
      baseURL: ctx.env.CLOUDFLARE_AIGATEWAY_URL,
    }),
    compat: new OpenAI({ apiKey: ctx.env.CLOUDFLARE_API_KEY, baseURL: ctx.env.CLOUDFLARE_AIGATEWAY_COMPAT_URL }),
  };
  const search = createSearch(ctx);

  // 실행당 워크스페이스 하나 — 스테이지가 이어받고, 이전 스테이지의 산출물은 확정되어
  // 읽기 전용으로 보인다. 상태는 도구 호출 열의 순수 함수라 리플레이가 재구성한다.
  const manuscriptFile = `manuscript/${ctx.document.refId}.txt`;
  const workspace = new Workspace([{ path: manuscriptFile, content: ctx.document.content, description: '검토 대상 원고' }]);

  await ctx.phase('research');
  const research = await runResearch(ctx, clients, workspace, manuscriptFile, search);
  const charter = renderResearchCharter(research.research);
  await ctx.ledger('research', research.research);
  await ctx.ledger('ledger/research', research.ledger);

  await ctx.phase('plan');
  const plan = await runPlan(ctx, clients, workspace, manuscriptFile, charter, search, research.ledger.tools);
  await ctx.ledger('plan', { final: plan.plan, rounds: plan.rounds });
  await ctx.ledger('ledger/plan', plan.ledger);

  await ctx.phase('execute');
  const execute = await runExecute(ctx, clients, workspace, manuscriptFile, charter, plan.plan, research.research);
  await ctx.ledger('execute', {
    findings: execute.findings,
    strengths: execute.strengths,
    discardedAxes: [...execute.discardedByAxis],
  });
  await ctx.ledger('ledger/execute', execute.ledger);

  await ctx.phase('local');
  // 겹침 소프트 경고의 기준 — 작품 검토가 접수한 앵커의 문자 범위.
  const executeAnchors = execute.findings.map((f) => ({ matchStart: f.matchStart, matchEnd: f.matchEnd }));
  const local = await runLocal(ctx, clients, workspace, manuscriptFile, charter, research.research, executeAnchors);
  await ctx.ledger('local', { findings: local.findings });
  await ctx.ledger('ledger/local', local.ledger);

  await ctx.phase('compose');
  const composed = await runCompose(
    ctx,
    clients,
    workspace,
    manuscriptFile,
    charter,
    plan.plan,
    research.research,
    [...execute.findings, ...local.findings],
    execute.strengths,
    execute.discardedByAxis,
  );
  // compose·composeReview의 사건은 각 스테이지 원장(ledger/compose·ledger/composeReview)에
  // 턴 기록과 함께 이미 실려 있다.

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
