import { finalizeHeader } from '../../../core/worker/deliverable.ts';
import { LLM_STEP } from '../../../core/worker/llm.ts';
import { checkEditorialPlan, FILE_REJECT_MAX, leakLineLint, mergeVerifications } from '../checks.ts';
import { PLAN_REVIEW_SCHEMA_V2, PLAN_SCHEMA } from '../contracts.ts';
import { emptyLedger } from '../ledger.ts';
import { renderPlanForReview, renderReviewFindingsForRevise, renderToolTrail } from '../render.ts';
import { runAgentStage } from './agent-stage.ts';
import type OpenAI from 'openai';
import type { PhasePrompt, ToolRecord, Usage } from '../../../core/contracts.ts';
import type { SearchExecutor } from '../../../core/worker/agent-loop.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { StageLedger } from '../ledger.ts';
import type { EditorialPlan, PlanReview } from '../types.ts';
import type { SubmitContext, SubmitOutcome } from './agent-stage.ts';

// 계획 검수 수렴 상한. 실측 75라운드에서 approve 0 — 이 검수는 상한이 얼마든 끝까지 쓰므로
// 상한이 곧 비용이다. 마지막 라운드의 발견도 수정에 반영되므로 축소의 손실은 추가 정제 1회분.
const PLAN_REVIEW_ROUNDS = 2;

const PLAN_PATH = 'output/plan.yaml';

const PLAN_DELIVERABLE: Deliverable = {
  label: '비평 계획',
  submitName: 'submit_plan',
  submitDescription:
    '비평 계획을 확정 제출한다. 검사를 통과해야 접수된다. 축 개수는 이 글의 위험 프로파일이 정한다 — 개수 자체를 조정 목표로 삼지 마라.',
  outputs: {
    [PLAN_PATH]: { schema: PLAN_SCHEMA, lints: [leakLineLint], description: '비평 계획 — 검토 축·보호 목록·확정 기록' },
  },
};

// 검수는 다른 벤더의 호출이다. GPT는 chat/completions에서 함수 도구와 추론을 함께 못 쓰므로
// structured output으로 받는다. 실패는 삼키지 않는다 — 스텝 실패로 표면화한다.
const callPlanReview = async (
  openai: OpenAI,
  prompt: PhasePrompt,
  system: string,
  userContent: string,
  usage: Usage,
): Promise<PlanReview> => {
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
  clients: LlmClients,
  workspace: Workspace,
  manuscriptFile: string,
  charter: string,
  search: SearchExecutor | null,
  researchTools: ToolRecord[],
): Promise<{ plan: EditorialPlan; ledger: StageLedger; rounds: { review: PlanReview }[] }> => {
  const content = ctx.document.content;
  const planLedger = emptyLedger();
  let planRejections = 0;

  const planSubmit =
    (prevPlan?: EditorialPlan) =>
    (_path: string, value: EditorialPlan, { turn, tools, ledger, file }: SubmitContext): SubmitOutcome<EditorialPlan> => {
      const check = checkEditorialPlan(
        content,
        prevPlan ? { ...value, verifications: mergeVerifications(prevPlan.verifications, value.verifications) } : value,
        tools,
        file,
      );
      for (const note of check.notes) ledger.events.push({ turn, kind: 'plan-check', detail: note });
      if (!check.contractOk && planRejections < FILE_REJECT_MAX) {
        planRejections += 1;
        return { reject: check.notes };
      }
      if (!check.contractOk) ledger.events.push({ turn, kind: 'plan-forced-accept', detail: '반려 상한 초과 — 마지막 제출 채택' });
      return { accept: check.plan, message: '접수.' };
    };

  // 수정 라운드가 같은 파일을 이어 편집한다 — 접수마다 확정하지 않고, 수렴 후 한 번 확정한다.
  const drafted = await runAgentStage<EditorialPlan, EditorialPlan>(ctx, {
    clients,
    stage: 'plan-draft',
    ledgerKey: 'plan',
    ledger: planLedger,
    prompt: ctx.prompts.plan,
    workspace,
    manuscriptAccess: true,
    system: charter,
    initial: `규약을 전제로 이 원고의 비평 계획을 ${PLAN_PATH}에 작성하고 submit_plan으로 제출하세요.`,
    search,
    deliverable: PLAN_DELIVERABLE,
    manuscriptFile,
    baseTools: researchTools,
    finalizeOnAccept: false,
    onSubmit: planSubmit(),
  });
  let plan = drafted.value;
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
    const notes = checkEditorialPlan(content, plan, allPlanTools, manuscriptFile).notes;
    const reviewInput = renderPlanForReview(plan, notes, renderToolTrail(allPlanTools));
    const review = (await ctx.step.do(`plan-review-${round}`, LLM_STEP, async () => {
      const { value } = await ctx.cached<PlanReview>(`plan/review/${round}`, (usage) =>
        callPlanReview(clients.compat, ctx.prompts.planReview, reviewSystem, reviewInput, usage),
      );
      return value as never;
    })) as unknown as PlanReview;

    rounds.push({ review });
    // 수렴 = approve 또는 차단 발견 부재. 적대 검수는 발견 생성을 멈추지 않으므로(15/15 미수렴 실측)
    // 종료 판정은 발견별 blocking 자기 신고에서 기계 유도한다. 권고만 남은 라운드는 수렴이다.
    const blocking = review.findings.filter((f) => f.blocking);
    // 검수 결과도 라운드마다 원장에 남긴다 — 수렴으로 루프를 빠져나가면 다음 스테이지의
    // 원장 갱신이 없어, 여기서 쓰지 않으면 마지막 검수가 화면에 도착하지 않는다.
    planLedger.events.push({
      turn: -1,
      kind: 'plan-review',
      detail: `라운드 ${round + 1}: ${review.verdict}, 발견 ${review.findings.length}건(차단 ${blocking.length})`,
    });
    await ctx.ledger('ledger/plan', planLedger);
    if (review.verdict === 'approve' || blocking.length === 0) {
      if (review.verdict !== 'approve') {
        planLedger.events.push({ turn: -1, kind: 'plan-review-converged', detail: `권고 ${review.findings.length}건만 남아 수렴` });
      }
      break;
    }

    // 차단 발견은 항상 수정 라운드를 받는다 — 마지막 라운드의 발견도 버려지지 않는다.
    await ctx.phase('plan');
    const revise = await runAgentStage<EditorialPlan, EditorialPlan>(ctx, {
      clients,
      stage: `plan-revise-${round}`,
      ledgerKey: 'plan',
      ledger: planLedger,
      prompt: ctx.prompts.plan,
      workspace,
      manuscriptAccess: true,
      system: charter,
      initial: [
        renderReviewFindingsForRevise(review.findings),
        '',
        `비평 계획이 ${PLAN_PATH}에 그대로 있습니다. 발견에 따라 edit로 수정하고 다시 submit_plan으로 제출하세요.`,
      ].join('\n'),
      search,
      deliverable: PLAN_DELIVERABLE,
      manuscriptFile,
      baseTools: [...researchTools, ...planLedger.tools],
      finalizeOnAccept: false,
      onSubmit: planSubmit(plan),
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

  // 라운드 수렴 후 한 번 확정 — 이후 스테이지에는 읽기 전용 산출물로 보인다.
  const spec = PLAN_DELIVERABLE.outputs[PLAN_PATH];
  workspace.finalize(PLAN_PATH, finalizeHeader(spec, PLAN_DELIVERABLE.label), spec.description);

  return { plan, ledger: planLedger, rounds };
};
