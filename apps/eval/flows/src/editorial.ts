import Anthropic from '@anthropic-ai/sdk';
import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import OpenAI from 'openai';
import { executeToolUses, runTurn } from './agent-loop.ts';
import { addUsage, cachedCall, cacheKey, callTool, LLM_STEP, setPhase } from './analysis-llm.ts';
import {
  AnalysisPromptSets,
  createDb,
  Documents,
  FeedbackAnchors,
  Feedbacks,
  FeedbackSets,
  PipelineRunDocs,
  writeStageCache,
} from './db.ts';
import {
  checkEditorialPlan,
  checkFinding,
  checkQuote,
  checkResearch,
  coverageGaps,
  FILE_REJECT_MAX,
  hasToolSyntaxLeak,
  LEAK_STREAK_MAX,
  mergeVerifications,
  TURN_CAP,
} from './editorial-checks.ts';
import {
  EDITORIAL_COMPOSE_REVIEW_TOOL,
  EDITORIAL_COMPOSE_TOOL,
  FILE_STRENGTH_TOOL,
  fileFindingTool,
  GREP_TOOL,
  LOCAL_AXES,
  PLAN_REVIEW_SCHEMA_V2,
  READ_TOOL,
  SEARCH_TOOL,
  SUBMIT_PLAN_TOOL,
  SUBMIT_RESEARCH_TOOL,
  SUBMIT_REVIEW_TOOL,
} from './editorial-contracts.ts';
import { emptyLedger } from './editorial-ledger.ts';
import {
  renderComposeInputV2,
  renderEditorialComposeReviewInput,
  renderEditorialPlanBlock,
  renderPlanForReview,
  renderRejection,
  renderResearchCharter,
  renderReviewFindingsForRevise,
  renderToolTrail,
} from './editorial-render.ts';
import { GateLedger, normalizeReviewItems } from './gates.ts';
import { renderSearchHits, searchBackground } from './search.ts';
import { createFindRange } from './text.ts';
import { schemaViolations } from './tool-schema.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { AnalysisStagePrompt } from '../../src/lib/domain/analysis-prompts.ts';
import type { SearchExecutor, ToolUse, TurnOutput } from './agent-loop.ts';
import type { Usage } from './analysis-llm.ts';
import type { Db } from './db.ts';
import type { StageLedger, ToolRecord } from './editorial-ledger.ts';
import type {
  AcceptedFinding,
  AcceptedStrength,
  EditorialFinding,
  EditorialPlan,
  EditorialStrength,
  PlanReview,
  ResolvedResearch,
} from './editorial-types.ts';
import type { EditorialParams, FlowEnv } from './index.ts';

// 직렬화 사고는 strict 스키마도 뚫는다(실측: protected 필드 누락 제출로 검증 크래시).
// 오형 제출은 처리 전에 걸러 반려 루프로 보낸다 — 검증 코드는 정형 입력만 전제한다.
const shapeRejection = (tool: { input_schema: unknown }, input: unknown): string[] | null => {
  const violations = schemaViolations(tool.input_schema, input);
  return violations.length > 0
    ? ['제출이 스키마와 다릅니다 — 필드 형태를 스키마 그대로 지켜 다시 제출하세요', ...violations.slice(0, 5)]
    : null;
};

// 계획 검수 수렴 상한 — v1.10에서 계승. approve만이 루프를 끝낸다.
const PLAN_REVIEW_ROUNDS = 3;

// 검수(OpenAI structured output). 실패는 삼키지 않는다 — 스텝 실패로 표면화한다.
const callPlanReviewV2 = async (
  env: FlowEnv,
  prompt: AnalysisStagePrompt,
  system: string,
  userContent: string,
  usage: Usage,
): Promise<PlanReview> => {
  const openai = new OpenAI({ apiKey: env.CLOUDFLARE_API_KEY, baseURL: env.CLOUDFLARE_AIGATEWAY_COMPAT_URL });
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: system },
      { role: 'user', content: userContent },
    ],
    response_format: { type: 'json_schema', json_schema: PLAN_REVIEW_SCHEMA_V2 as never },
  };
  if (prompt.effort) (params as Record<string, unknown>).reasoning_effort = prompt.effort;

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

// 제출 처리 결과. done이 설정되면 이 턴의 나머지 결과를 반영한 뒤 단계를 끝낸다.
type SubmissionOutcome<T> = { done?: T; results: { toolUseId: string; content: string }[] };

type AgentStageOptions<T> = {
  step: WorkflowStep;
  db: Db;
  client: Anthropic;
  runId: string;
  documentId: string;
  content: string;
  stage: string;
  prompt: AnalysisStagePrompt;
  tools: Anthropic.Messages.Tool[];
  system: string;
  initial: string;
  search: SearchExecutor | null;
  // 이전 단계에서 넘어온 도구 기록 — 열람 범위 검증이 단계를 넘어 이어진다(계획 초안에서 읽은
  // 대목을 수정 라운드에서 인용하는 경우).
  baseTools: ToolRecord[];
  onSubmissions: (subs: ToolUse[], turn: number, tools: ToolRecord[], ledger: StageLedger) => SubmissionOutcome<T>;
};

// 에이전틱 단계의 공통 루프. 턴 하나 = 스텝 하나, 턴당 D1 캐시. 원장은 캐시된 도구 실행
// 결과에서 매 리플레이 재구성된다 — 순수해야 하는 이유다.
const runAgentStage = async <T>(options: AgentStageOptions<T>): Promise<{ value: T; ledger: StageLedger }> => {
  const { step, db, client, runId, documentId, content, stage, prompt, tools, system, initial, search, baseTools, onSubmissions } = options;
  const ledger = emptyLedger();
  const messages: Anthropic.MessageParam[] = [{ role: 'user', content: initial }];
  let leakStreak = 0;

  for (let turn = 0; turn < TURN_CAP; turn++) {
    const turnStep = await step.do(`${stage}-${turn}`, LLM_STEP, async () => {
      const { value, usage, cached } = await cachedCall<TurnOutput>(db, runId, documentId, `${stage}/turn/${turn}`, (usage) =>
        runTurn(client, prompt, tools, system, messages, usage),
      );
      await addUsage(db, runId, documentId, stage, usage);
      return { out: value, cached };
    });
    const out = turnStep.out;
    messages.push({ role: 'assistant', content: out.content as Anthropic.MessageParam['content'] });

    if (out.toolUses.length === 0) {
      messages.push({ role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' });
      continue;
    }

    // 검색이 비결정적이므로 도구 실행 전체를 캐시한다 — 리플레이가 같은 결과를 재사용한다.
    const executed = await step.do(`${stage}-tools-${turn}`, async () => {
      const { value } = await cachedCall(db, runId, documentId, `${stage}/tools/${turn}`, () =>
        executeToolUses(content, out.toolUses, turn, search),
      );
      return value;
    });
    ledger.tools.push(...executed.records);

    const combinedTools = [...baseTools, ...ledger.tools];

    // 직렬화 오염 제출은 핸들러 앞에서 중앙 차단한다. 오염 문면이 대화에 남으면 이후 제출이
    // 그 형태를 모방하므로 원문을 컨텍스트에서 제거하고 처음부터 다시 쓰게 한다. 연속 오염은
    // 스테이지를 중단해 턴 낭비를 끊는다 — 회수는 캐시 리플레이 재실행.
    const cleanSubs: ToolUse[] = [];
    const leakResults: { toolUseId: string; content: string }[] = [];
    for (const sub of executed.submissions) {
      if (!hasToolSyntaxLeak([JSON.stringify(sub.input)])) {
        leakStreak = 0;
        cleanSubs.push(sub);
        continue;
      }
      leakStreak += 1;
      const input = sub.input as Record<string, unknown>;
      const excerpt = String(typeof input?.quoteStart === 'string' ? input.quoteStart : (input?.intent ?? '')).slice(0, 30);
      ledger.events.push({ turn, kind: 'leak-rejected', detail: `${sub.name}: ${excerpt}` });
      ledger.leaked.push({ turn, name: sub.name, input: sub.input });
      // 라이브 턴에서만 중단한다. 리플레이(캐시 턴)에서 발동하면 자력 회복하고 완주했던 실행의
      // 회수가 영구히 막힌다 — 실측: 7연속 오염 후 회복·완주 사례 존재.
      if (leakStreak > LEAK_STREAK_MAX && !turnStep.cached) {
        throw new Error(`${stage}: 직렬화 오염 연속 ${leakStreak}회 — 컨텍스트 오염으로 스테이지 중단`);
      }
      leakResults.push({
        toolUseId: sub.id,
        content: renderRejection([
          '제출 필드에 도구 호출 구문이 혼입되어 원문을 대화에서 제거했습니다',
          `제출 식별: ${excerpt || sub.name}`,
          '직전 제출 문면을 참조하지 말고, 같은 내용을 처음부터 순수 텍스트로 새로 작성해 제출하세요',
        ]),
      });
      const last = messages.at(-1);
      if (last?.role === 'assistant' && Array.isArray(last.content)) {
        for (const block of last.content) {
          if (block.type === 'tool_use' && block.id === sub.id) {
            block.input = { scrubbed: '직렬화 오염 제출 — 원문 제거됨' };
          }
        }
      }
    }

    const outcome = onSubmissions(cleanSubs, turn, combinedTools, ledger);

    const resultOf = new Map<string, string>();
    for (const r of executed.results) resultOf.set(r.toolUseId, r.content);
    for (const r of leakResults) resultOf.set(r.toolUseId, r.content);
    for (const r of outcome.results) resultOf.set(r.toolUseId, r.content);
    messages.push({
      role: 'user',
      content: out.toolUses.map((use) => ({
        type: 'tool_result' as const,
        tool_use_id: use.id,
        content: resultOf.get(use.id) ?? '처리되지 않은 호출',
      })),
    });

    if (outcome.done !== undefined) return { value: outcome.done, ledger };
  }

  throw new Error(`${stage}: 턴 백스톱(${TURN_CAP}) 초과`);
};

export class EditorialWorkflow extends WorkflowEntrypoint<FlowEnv, EditorialParams> {
  async run(event: WorkflowEvent<EditorialParams>, step: WorkflowStep) {
    const { runId, promptSetId, documentId } = event.payload;
    const db = createDb(this.env.DB);
    const client = new Anthropic({ apiKey: null, authToken: this.env.CLOUDFLARE_API_KEY, baseURL: this.env.CLOUDFLARE_AIGATEWAY_URL });

    const resolved = await step.do('resolve', async () => {
      const [doc] = await db.select().from(Documents).where(eq(Documents.id, documentId));
      if (!doc) throw new Error('document not found');
      const [set] = await db.select().from(AnalysisPromptSets).where(eq(AnalysisPromptSets.id, promptSetId));
      if (!set) throw new Error('prompt set not found');
      const prompts = set.content;
      const required = {
        research: prompts.research,
        plan: prompts.plan,
        planReview: prompts.planReview,
        execute: prompts.execute,
        local: prompts.local,
        compose: prompts.compose,
        composeReview: prompts.composeReview,
      };
      for (const [key, value] of Object.entries(required)) {
        if (!value) throw new Error(`editorial prompt set requires ${key}`);
      }
      await db
        .update(PipelineRunDocs)
        .set({ status: 'running', phase: 'research', error: null })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      return { content: doc.content, prompts };
    });

    const { content, prompts } = resolved;
    const search: SearchExecutor | null = this.env.EXA_API_KEY
      ? async (query) => {
          const hits = await searchBackground({ apiKey: this.env.EXA_API_KEY as string, query });
          return { content: hits.length > 0 ? renderSearchHits(hits) : '검색 결과 없음', hits: hits.length };
        }
      : null;

    const stageBase = { step, db, client, runId, documentId, content } as const;

    try {
      // ── RESEARCH ── 규약의 확정. 제출은 반려하지 않고 정제+기록한다(스펙 §5 — 자유 탐색).
      const researchStage = await runAgentStage<ResolvedResearch>({
        ...stageBase,
        stage: 'research',
        prompt: prompts.research as AnalysisStagePrompt,
        tools: [READ_TOOL, GREP_TOOL, SEARCH_TOOL, SUBMIT_RESEARCH_TOOL],
        system: '',
        initial: '원고를 조사하고 submit_research로 제출하세요.',
        search,
        baseTools: [],
        onSubmissions: (subs, turn, tools, ledger) => {
          const results: { toolUseId: string; content: string }[] = [];
          let done: ResolvedResearch | undefined;
          for (const sub of subs) {
            const shape = shapeRejection(SUBMIT_RESEARCH_TOOL, sub.input);
            if (shape) {
              ledger.events.push({ turn, kind: 'research-check', detail: `오형 제출 반려: ${shape[1] ?? ''}`.slice(0, 120) });
              results.push({ toolUseId: sub.id, content: renderRejection(shape) });
              continue;
            }
            // 오염된 규약은 전 하류를 오염시킨다 — 조사 제출도 직렬화 사고를 반려한다.
            if (hasToolSyntaxLeak([JSON.stringify(sub.input)])) {
              ledger.events.push({ turn, kind: 'research-check', detail: '제출에 도구 구문 누출 — 반려' });
              results.push({
                toolUseId: sub.id,
                content: renderRejection(['필드에 도구 호출 구문이 섞임 — 각 필드를 순수 텍스트로 다시 제출하라']),
              });
              continue;
            }
            const check = checkResearch(content, sub.input as never, tools);
            for (const note of check.notes) ledger.events.push({ turn, kind: 'research-check', detail: note });
            done = check.research;
            results.push({ toolUseId: sub.id, content: `접수. 코드 검증 ${check.notes.length}건 정제.` });
          }
          return { done, results };
        },
      });
      const research = researchStage.value;
      const charter = renderResearchCharter(research);
      await step.do('research-persist', async () => {
        await writeStageCache(db, cacheKey(runId, documentId, 'research'), research);
        await writeStageCache(db, cacheKey(runId, documentId, 'ledger/research'), researchStage.ledger);
      });

      // ── PLAN ── 초안 → (코드 검증 → 검수 → 수정)*. v1.10의 수렴 구조 + 도구.
      await step.do('phase-plan', () => setPhase(db, runId, documentId, 'plan'));
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

      const draft = await runAgentStage<EditorialPlan>({
        ...stageBase,
        stage: 'plan-draft',
        prompt: prompts.plan as AnalysisStagePrompt,
        tools: planTools,
        system: charter,
        initial: '규약을 전제로 이 원고의 비평 계획을 세워 submit_plan으로 제출하세요.',
        search,
        baseTools: researchStage.ledger.tools,
        onSubmissions: planSubmissionHandler(),
      });
      planLedger.tools.push(...draft.ledger.tools);
      planLedger.events.push(...draft.ledger.events);
      let plan = draft.value;
      const planRounds: { review: PlanReview }[] = [];

      const reviewSystem = [
        (prompts.planReview as AnalysisStagePrompt).system,
        '',
        charter,
        '',
        '당신에게 원고 접근은 없다. 계획·조사 기록·규약이 판정 근거의 전부다.',
      ].join('\n');

      for (let round = 0; round < PLAN_REVIEW_ROUNDS; round++) {
        const allPlanTools = [...researchStage.ledger.tools, ...planLedger.tools];
        const notes = checkEditorialPlan(content, plan, allPlanTools).notes;
        const reviewInput = renderPlanForReview(plan, notes, renderToolTrail(allPlanTools));
        const review = await step.do(`plan-review-${round}`, LLM_STEP, async () => {
          const { value, usage } = await cachedCall<PlanReview>(db, runId, documentId, `plan/review/${round}`, (usage) =>
            callPlanReviewV2(this.env, prompts.planReview as AnalysisStagePrompt, reviewSystem, reviewInput, usage),
          );
          await addUsage(db, runId, documentId, 'planReview', usage);
          return value;
        });
        planRounds.push({ review });
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
        const revise = await runAgentStage<EditorialPlan>({
          ...stageBase,
          stage: `plan-revise-${round}`,
          prompt: prompts.plan as AnalysisStagePrompt,
          tools: planTools,
          system: charter,
          // 계획 직렬화는 압축한다 — 들여쓰기 공백이 라운드마다 입력 토큰의 3~4할을 먹는다.
          initial: ['<원래 계획>', JSON.stringify(plan), '</원래 계획>', '', renderReviewFindingsForRevise(review.findings)].join('\n'),
          search,
          baseTools: [...researchStage.ledger.tools, ...planLedger.tools],
          onSubmissions: planSubmissionHandler(plan),
        });
        planLedger.tools.push(...revise.ledger.tools);
        planLedger.events.push(...revise.ledger.events);
        plan = revise.value;
        if (round === PLAN_REVIEW_ROUNDS - 1) {
          planLedger.events.push({
            turn: -1,
            kind: 'plan-review-exhausted',
            detail: `검수 ${PLAN_REVIEW_ROUNDS}회 소진 — 마지막 발견까지 수정에 반영됨`,
          });
        }
      }

      await step.do('plan-persist', async () => {
        await writeStageCache(db, cacheKey(runId, documentId, 'plan'), { final: plan, rounds: planRounds });
        await writeStageCache(db, cacheKey(runId, documentId, 'ledger/plan'), planLedger);
      });

      // ── EXECUTE ── 순차 통독 + 증분 제출. 커버리지 게이트가 종료를 지킨다.
      await step.do('phase-execute', () => setPhase(db, runId, documentId, 'execute'));
      const axisLabels = plan.axes.map((a) => a.label);
      const executeFindingTool = fileFindingTool(axisLabels);
      const findings: AcceptedFinding[] = [];
      const strengths: AcceptedStrength[] = [];
      const rejectCounts = new Map<string, number>();
      // 제출됐다가 유실된 지적의 축 — 지적 0건과 구별해 총평 cleared(무혐의)에서 제외한다.
      const discardedByAxis = new Map<string, number>();
      const findRange = createFindRange(content);

      const executeStage = await runAgentStage<true>({
        ...stageBase,
        stage: 'execute',
        prompt: prompts.execute as AnalysisStagePrompt,
        tools: [READ_TOOL, GREP_TOOL, executeFindingTool, FILE_STRENGTH_TOOL, SUBMIT_REVIEW_TOOL],
        system: [charter, '', renderEditorialPlanBlock(plan)].join('\n'),
        initial:
          '원고를 처음부터 순서대로 읽으세요. 읽다가 걸리면 그 자리에서 file_finding으로, 잘 작동하는 대목은 file_strength로 제출하세요. 끝까지 읽고 확인이 끝나면 submit_review로 마치세요.',
        search: null,
        baseTools: [],
        onSubmissions: (subs, turn, tools, ledger) => {
          const results: { toolUseId: string; content: string }[] = [];
          let done: true | undefined;
          for (const sub of subs) {
            if (sub.name === 'file_finding') {
              const shape = shapeRejection(executeFindingTool, sub.input);
              if (shape) {
                ledger.events.push({ turn, kind: 'finding-rejected', detail: `오형 제출: ${shape[1] ?? ''}`.slice(0, 120) });
                results.push({ toolUseId: sub.id, content: renderRejection(shape) });
                continue;
              }
              const finding = sub.input as EditorialFinding;
              const check = checkFinding(content, finding, tools, turn, axisLabels, research.boundaryRanges);
              if (check.accepted) {
                findings.push(check.accepted);
                results.push({ toolUseId: sub.id, content: `접수 (${findings.length}건째).` });
                continue;
              }
              const key = finding.quoteStart.slice(0, 20);
              const count = (rejectCounts.get(key) ?? 0) + 1;
              rejectCounts.set(key, count);
              for (const reason of check.reasons) ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
              if (count > FILE_REJECT_MAX) {
                ledger.events.push({ turn, kind: 'finding-discarded', detail: `${finding.axis} — ${key}` });
                discardedByAxis.set(finding.axis, (discardedByAxis.get(finding.axis) ?? 0) + 1);
                results.push({ toolUseId: sub.id, content: '반려 상한 초과 — 이 지적은 폐기합니다. 다음으로 진행하세요.' });
              } else {
                results.push({ toolUseId: sub.id, content: renderRejection(check.reasons) });
              }
              continue;
            }
            if (sub.name === 'file_strength') {
              const s = sub.input as EditorialStrength;
              const range = findRange(s.quoteStart, s.quoteEnd, 0);
              const quoteOk = range !== null && checkQuote(content, tools, s.quoteStart).range !== null;
              if (range && quoteOk) {
                strengths.push({ ...s, matchStart: range.rangeStart, matchEnd: range.rangeEnd });
                results.push({ toolUseId: sub.id, content: '접수.' });
              } else {
                results.push({ toolUseId: sub.id, content: renderRejection(['강점 앵커가 원고에 없거나 열람 범위 밖']) });
              }
              continue;
            }
            if (sub.name === 'submit_review') {
              const gaps = coverageGaps(content.length, tools, research.boundaryRanges);
              if (gaps.length > 0) {
                results.push({
                  toolUseId: sub.id,
                  content: renderRejection([`미열람 구간이 남았습니다: ${gaps.map((g) => `${g.start}~${g.end}`).join(', ')}`]),
                });
              } else {
                done = true;
                results.push({ toolUseId: sub.id, content: '검토 종료.' });
              }
              continue;
            }
            results.push({ toolUseId: sub.id, content: '알 수 없는 제출' });
          }
          return { done, results };
        },
      });

      // 중앙 차단으로 제거된 지적 제출도 유실이다 — 축이 식별되면 해당 축만, 아니면 어느 축의
      // 발견이 사라졌는지 알 수 없으므로 cleared 판정에서 그 축을 가려낼 수 없음을 원장에 남긴다.
      for (const leak of executeStage.ledger.leaked) {
        if (leak.name !== 'file_finding') continue;
        const axis = (leak.input as { axis?: unknown })?.axis;
        if (typeof axis === 'string' && axisLabels.includes(axis)) {
          discardedByAxis.set(axis, (discardedByAxis.get(axis) ?? 0) + 1);
        }
      }

      await step.do('execute-persist', async () => {
        await writeStageCache(db, cacheKey(runId, documentId, 'execute'), {
          findings,
          strengths,
          discardedAxes: [...discardedByAxis],
        });
        await writeStageCache(db, cacheKey(runId, documentId, 'ledger/execute'), executeStage.ledger);
      });

      // ── LOCAL ── 문면 층위(문장 결·원고 사고)의 단독 소유자. 접근 방식은 회수와 무관함이
      // 실측됐고(주입 6/13 vs read 5/13), 회수를 만드는 것은 층위 전용 범위다.
      await step.do('phase-local', () => setPhase(db, runId, documentId, 'local'));
      const localStart = findings.length;
      const localFindingTool = fileFindingTool(LOCAL_AXES);
      const localRejects = new Map<string, number>();
      let localNudged = false;
      const localStage = await runAgentStage<true>({
        ...stageBase,
        stage: 'local',
        prompt: prompts.local as AnalysisStagePrompt,
        tools: [READ_TOOL, GREP_TOOL, localFindingTool, SUBMIT_REVIEW_TOOL],
        system: charter,
        initial:
          '원고를 처음부터 끝까지 read로 통독하며 문면 층위를 검토하세요. 걸리면 그 자리에서 file_finding으로 제출하고, 끝나면 submit_review로 마치세요.',
        search: null,
        baseTools: [],
        onSubmissions: (subs, turn, tools, ledger) => {
          const results: { toolUseId: string; content: string }[] = [];
          let done: true | undefined;
          for (const sub of subs) {
            if (sub.name === 'file_finding') {
              const shape = shapeRejection(localFindingTool, sub.input);
              if (shape) {
                ledger.events.push({ turn, kind: 'finding-rejected', detail: `오형 제출: ${shape[1] ?? ''}`.slice(0, 120) });
                results.push({ toolUseId: sub.id, content: renderRejection(shape) });
                continue;
              }
              const finding = sub.input as EditorialFinding;
              const check = checkFinding(content, finding, tools, turn, LOCAL_AXES, research.boundaryRanges);
              if (check.accepted) {
                findings.push(check.accepted);
                results.push({ toolUseId: sub.id, content: '접수.' });
                continue;
              }
              const key = finding.quoteStart.slice(0, 20);
              const count = (localRejects.get(key) ?? 0) + 1;
              localRejects.set(key, count);
              for (const reason of check.reasons) ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
              if (count > FILE_REJECT_MAX) {
                ledger.events.push({ turn, kind: 'finding-discarded', detail: key });
                results.push({ toolUseId: sub.id, content: '반려 상한 초과 — 이 지적은 폐기합니다. 다음으로 진행하세요.' });
              } else {
                results.push({ toolUseId: sub.id, content: renderRejection(check.reasons) });
              }
              continue;
            }
            if (sub.name === 'submit_review') {
              const gaps = coverageGaps(content.length, tools, research.boundaryRanges);
              if (gaps.length > 0) {
                results.push({
                  toolUseId: sub.id,
                  content: renderRejection([`미열람 구간이 남았습니다: ${gaps.map((g) => `${g.start}~${g.end}`).join(', ')}`]),
                });
                continue;
              }
              // 실측된 유실 모드(대조까지 하고 미제출) 조준 — 종료 전 정확히 한 번 되묻는다.
              if (!localNudged) {
                localNudged = true;
                results.push({
                  toolUseId: sub.id,
                  content:
                    '제출 전 확인: 대조로 확인했지만 제출하지 않은 후보가 남아 있으면 지금 file_finding으로 제출하세요. 없으면 submit_review를 다시 호출해 마치세요.',
                });
                continue;
              }
              done = true;
              results.push({ toolUseId: sub.id, content: '검토 종료.' });
              continue;
            }
            results.push({ toolUseId: sub.id, content: '알 수 없는 제출' });
          }
          return { done, results };
        },
      });

      await step.do('local-persist', async () => {
        await writeStageCache(db, cacheKey(runId, documentId, 'local'), { findings: findings.slice(localStart) });
        await writeStageCache(db, cacheKey(runId, documentId, 'ledger/local'), localStage.ledger);
      });

      // ── COMPOSE ── 문면 확정. category는 축에서 파생한다 — 지적→축→계획의 추적선.
      await step.do('phase-compose', () => setPhase(db, runId, documentId, 'compose'));
      const composeOptions = { conventions: [charter, '', renderEditorialPlanBlock(plan)].join('\n'), cache: false };

      // 원고 순서로 정렬해 넘긴다 — 작성자가 좌표 없이 순서를 유지하면 된다.
      const ordered = findings.toSorted((a, b) => a.matchStart - b.matchStart);
      const composed = await step.do('compose', LLM_STEP, async () => {
        const { value, usage } = await cachedCall(db, runId, documentId, 'compose', async (usage) => {
          type Raw = { feedbacks: { findingIndexes: number[]; body: string }[] };
          const raw = await callTool<Raw>(
            client,
            prompts.compose as AnalysisStagePrompt,
            EDITORIAL_COMPOSE_TOOL,
            renderComposeInputV2(ordered),
            usage,
            composeOptions,
          );

          const events: { kind: string; detail: string }[] = [];
          const seen = new Set<number>();
          const delivered: {
            category: string;
            body: string;
            anchors: { quoteStart: string; quoteEnd: string; matchStart: number; matchEnd: number }[];
          }[] = [];
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
              if (axis === majorityAxis) {
                continue;
              }

              for (const i of indexes) seen.delete(i);
              events.push({ kind: 'compose-mixed-axis', detail: `${axis}: ${indexes.join(',')}` });
            }
            delivered.push({
              category: majorityAxis,
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
        await addUsage(db, runId, documentId, 'compose', usage);
        return value;
      });

      const reviewStep = await step.do('compose-review', LLM_STEP, async () => {
        const { value, usage } = await cachedCall(db, runId, documentId, 'compose-review', async (usage) => {
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
            prompts.composeReview as AnalysisStagePrompt,
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
            review: JSON.stringify({
              ...raw,
              strengths: resolvedStrengths,
              cleared,
              patterns: normalizeReviewItems(raw.patterns ?? [], composed.delivered.length, gateLedger),
              priority: normalizeReviewItems(raw.priority ?? [], composed.delivered.length, gateLedger),
            }),
          };
        });
        await addUsage(db, runId, documentId, 'composeReview', usage);
        return value;
      });

      // ── 저장 ── 산출물 계약 불변 — 기존 스키마 그대로.
      await step.do('persist', async () => {
        const review = JSON.parse(reviewStep.review) as Record<string, unknown>;
        const [existing] = await db
          .select({ id: FeedbackSets.id })
          .from(FeedbackSets)
          .where(and(eq(FeedbackSets.runId, runId), eq(FeedbackSets.documentId, documentId)));
        const setId = existing?.id ?? nanoid();
        if (existing) {
          await db.update(FeedbackSets).set({ review }).where(eq(FeedbackSets.id, setId));
          // 완료된 실행의 리플레이(총평만 재생성 등)로 다시 오는 경로다 — 지난 삽입분을 지워
          // 멱등으로 만든다. 피드백 id는 바뀌므로 판정이 걸린 세트에는 쓰지 말 것.
          const stale = await db.select({ id: Feedbacks.id }).from(Feedbacks).where(eq(Feedbacks.setId, setId));
          if (stale.length > 0) {
            await db.delete(FeedbackAnchors).where(
              inArray(
                FeedbackAnchors.feedbackId,
                stale.map((f) => f.id),
              ),
            );
            await db.delete(Feedbacks).where(eq(Feedbacks.setId, setId));
          }
        } else {
          await db.insert(FeedbackSets).values({ id: setId, runId, documentId, variantId: event.payload.variantLabel, review });
        }

        for (const [ord, feedback] of composed.delivered.entries()) {
          const feedbackId = nanoid();
          const head = feedback.anchors[0];
          await db.insert(Feedbacks).values({
            id: feedbackId,
            setId,
            ord,
            startText: head?.quoteStart ?? '',
            endText: head?.quoteEnd ?? '',
            matchStart: head?.matchStart ?? null,
            matchEnd: head?.matchEnd ?? null,
            category: feedback.category,
            polarity: 'issue',
            layer: (LOCAL_AXES as string[]).includes(feedback.category) ? 'local' : 'plan',
            body: feedback.body,
          });
          for (const [anchorOrd, anchor] of feedback.anchors.entries()) {
            await db.insert(FeedbackAnchors).values({
              id: nanoid(),
              feedbackId,
              ord: anchorOrd,
              startText: anchor.quoteStart,
              endText: anchor.quoteEnd,
              matchStart: anchor.matchStart,
              matchEnd: anchor.matchEnd,
            });
          }
        }

        await writeStageCache(db, cacheKey(runId, documentId, 'ledger/compose'), composed.events);
        await db
          .update(PipelineRunDocs)
          .set({ status: 'done', phase: 'done', doneChunks: composed.delivered.length, totalChunks: composed.delivered.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { feedbacks: composed.delivered.length };
    } catch (err) {
      const message = String(err).slice(0, 1000);
      await step.do('mark-failed', async () => {
        await db
          .update(PipelineRunDocs)
          .set({ status: 'failed', error: message })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });
      throw err;
    }
  }
}
