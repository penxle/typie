import OpenAI from 'openai';
import { LLM_STEP } from '../../../core/worker/llm.ts';
import { checkEditorialPlan, FILE_REJECT_MAX, mergeVerifications } from '../checks.ts';
import { GREP_TOOL, PLAN_REVIEW_SCHEMA_V2, READ_TOOL, SEARCH_TOOL, SUBMIT_PLAN_TOOL } from '../contracts.ts';
import { emptyLedger } from '../ledger.ts';
import { renderPlanForReview, renderRejection, renderReviewFindingsForRevise, renderToolTrail } from '../render.ts';
import { runAgentStage, shapeRejection } from './agent-stage.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { PhasePrompt, ToolRecord, Usage } from '../../../core/contracts.ts';
import type { SearchExecutor, ToolUse } from '../../../core/worker/agent-loop.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { StageLedger } from '../ledger.ts';
import type { EditorialPlan, PlanReview } from '../types.ts';
import type { SubmissionOutcome } from './agent-stage.ts';

// 계획 검수 수렴 상한. 실측 75라운드에서 approve 0 — 이 검수는 상한이 얼마든 끝까지 쓰므로
// 상한이 곧 비용이다. 마지막 라운드의 발견도 수정에 반영되므로 축소의 손실은 추가 정제 1회분.
const PLAN_REVIEW_ROUNDS = 2;

// 검수는 다른 벤더의 호출이다. GPT는 chat/completions에서 함수 도구와 추론을 함께 못 쓰므로
// structured output으로 받는다. 실패는 삼키지 않는다 — 스텝 실패로 표면화한다.
const callPlanReview = async (
  ctx: RunContext,
  prompt: PhasePrompt,
  system: string,
  userContent: string,
  usage: Usage,
): Promise<PlanReview> => {
  const openai = new OpenAI({ apiKey: ctx.env.CLOUDFLARE_API_KEY, baseURL: ctx.env.CLOUDFLARE_AIGATEWAY_COMPAT_URL });
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: system },
      { role: 'user', content: userContent },
    ],
    response_format: { type: 'json_schema', json_schema: PLAN_REVIEW_SCHEMA_V2 as never },
  };
  if (prompt.effort) (params as unknown as Record<string, unknown>).reasoning_effort = prompt.effort;

  const res = await openai.chat.completions.create(params, { headers: { 'cf-aig-skip-cache': 'true' } });
  usage.calls += 1;
  usage.promptTokens += res.usage?.prompt_tokens ?? 0;
  usage.cachedTokens += res.usage?.prompt_tokens_details?.cached_tokens ?? 0;
  usage.completionTokens += res.usage?.completion_tokens ?? 0;

  const parsed = JSON.parse(res.choices[0]?.message?.content ?? '') as PlanReview;
  if (parsed.verdict !== 'approve' && parsed.verdict !== 'needs-attention') {
    throw new Error(`plan_review: verdict 위반 — ${String(parsed.verdict)}`);
  }
  return parsed;
};

// 초안 → (코드 검증 → 검수 → 수정)*. 검수는 원고 접근 없이 계획·조사 기록·규약만 본다.
export const runPlan = async (
  ctx: RunContext,
  client: Anthropic,
  charter: string,
  search: SearchExecutor | null,
  researchTools: ToolRecord[],
): Promise<{ plan: EditorialPlan; ledger: StageLedger; rounds: { review: PlanReview }[] }> => {
  const content = ctx.document.content;
  const planTools = [READ_TOOL, GREP_TOOL, SEARCH_TOOL, SUBMIT_PLAN_TOOL];
  const planLedger = emptyLedger();
  let planRejections = 0;

  const planSubmissionHandler =
    (prevPlan?: EditorialPlan) =>
    (subs: ToolUse[], turn: number, tools: ToolRecord[], ledger: StageLedger): SubmissionOutcome<EditorialPlan> => {
      const results: { toolUseId: string; content: string }[] = [];
      let done: EditorialPlan | undefined;
      for (const sub of subs) {
        const shape = shapeRejection(SUBMIT_PLAN_TOOL, sub.input);
        if (shape) {
          ledger.events.push({ turn, kind: 'plan-check', detail: `오형 제출 반려: ${shape[1] ?? ''}`.slice(0, 120) });
          results.push({ toolUseId: sub.id, content: renderRejection(shape) });
          continue;
        }
        const submitted = sub.input as EditorialPlan;
        const check = checkEditorialPlan(
          content,
          prevPlan ? { ...submitted, verifications: mergeVerifications(prevPlan.verifications, submitted.verifications) } : submitted,
          tools,
        );
        for (const note of check.notes) ledger.events.push({ turn, kind: 'plan-check', detail: note });
        if (!check.contractOk && planRejections < FILE_REJECT_MAX) {
          planRejections += 1;
          results.push({ toolUseId: sub.id, content: renderRejection(check.notes) });
          continue;
        }
        if (!check.contractOk) ledger.events.push({ turn, kind: 'plan-forced-accept', detail: '반려 상한 초과 — 마지막 제출 채택' });
        done = check.plan;
        results.push({ toolUseId: sub.id, content: '접수.' });
      }
      return { done, results };
    };

  const draft = await runAgentStage<EditorialPlan>(ctx, {
    client,
    stage: 'plan-draft',
    ledgerKey: 'plan',
    ledger: planLedger,
    prompt: ctx.prompts.plan,
    tools: planTools,
    system: charter,
    initial: '규약을 전제로 이 원고의 비평 계획을 세워 submit_plan으로 제출하세요.',
    search,
    baseTools: researchTools,
    onSubmissions: planSubmissionHandler(),
  });
  let plan = draft.value;
  const rounds: { review: PlanReview }[] = [];

  const reviewSystem = [
    ctx.prompts.planReview.system,
    '',
    charter,
    '',
    '당신에게 원고 접근은 없다. 계획·조사 기록·규약이 판정 근거의 전부다.',
  ].join('\n');

  for (let round = 0; round < PLAN_REVIEW_ROUNDS; round++) {
    await ctx.phase('planReview');
    const allPlanTools = [...researchTools, ...planLedger.tools];
    const notes = checkEditorialPlan(content, plan, allPlanTools).notes;
    const reviewInput = renderPlanForReview(plan, notes, renderToolTrail(allPlanTools));
    const review = (await ctx.step.do(`plan-review-${round}`, LLM_STEP, async () => {
      const { value } = await ctx.cached<PlanReview>(`plan/review/${round}`, (usage) =>
        callPlanReview(ctx, ctx.prompts.planReview, reviewSystem, reviewInput, usage),
      );
      return value as never;
    })) as unknown as PlanReview;

    rounds.push({ review });
    // 수렴 = approve 또는 차단 발견 부재. 적대 검수는 발견 생성을 멈추지 않으므로(15/15 미수렴 실측)
    // 종료 판정은 발견별 blocking 자기 신고에서 기계 유도한다. 권고만 남은 라운드는 수렴이다.
    const blocking = review.findings.filter((f) => f.blocking);
    if (review.verdict === 'approve' || blocking.length === 0) {
      if (review.verdict !== 'approve') {
        planLedger.events.push({ turn: -1, kind: 'plan-review-converged', detail: `권고 ${review.findings.length}건만 남아 수렴` });
      }
      break;
    }

    // 차단 발견은 항상 수정 라운드를 받는다 — 마지막 라운드의 발견도 버려지지 않는다.
    await ctx.phase('plan');
    const revise = await runAgentStage<EditorialPlan>(ctx, {
      client,
      stage: `plan-revise-${round}`,
      ledgerKey: 'plan',
      ledger: planLedger,
      prompt: ctx.prompts.plan,
      tools: planTools,
      system: charter,
      // 계획 직렬화는 압축한다 — 들여쓰기 공백이 라운드마다 입력 토큰의 3~4할을 먹는다.
      initial: ['<원래 계획>', JSON.stringify(plan), '</원래 계획>', '', renderReviewFindingsForRevise(review.findings)].join('\n'),
      search,
      baseTools: [...researchTools, ...planLedger.tools],
      onSubmissions: planSubmissionHandler(plan),
    });
    plan = revise.value;
    if (round === PLAN_REVIEW_ROUNDS - 1) {
      planLedger.events.push({
        turn: -1,
        kind: 'plan-review-exhausted',
        detail: `검수 ${PLAN_REVIEW_ROUNDS}회 소진 — 마지막 발견까지 수정에 반영됨`,
      });
    }
  }

  return { plan, ledger: planLedger, rounds };
};
