import Anthropic from '@anthropic-ai/sdk';
import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import OpenAI from 'openai';
import {
  addUsage,
  cachedCall,
  cachedStep,
  cacheKey,
  callTool,
  emptyUsage,
  LLM_STEP,
  setPhase,
  sumUsage,
  warmPrefix,
} from './analysis-llm.ts';
import {
  renderBackgroundInput,
  renderComposeInput,
  renderComposeReviewInput,
  renderConventions,
  renderDedupeInput,
  renderPlan,
  renderPlanReviewInput,
  renderPlanReviseInput,
  renderProfile,
  renderReviewInput,
  renderScenes,
  renderSelfCheckInput,
  renderVerifyInput,
} from './analysis-render.ts';
import {
  BACKGROUND_TOOL,
  COMPOSE_REVIEW_TOOL,
  COMPOSE_TOOL,
  DEDUPE_TOOL,
  GENRE_TOOL,
  PLAN_TOOL,
  REVIEW_TOOL,
  reviewToolWithAxes,
  SELFCHECK_TOOL,
  SURVEY_TOOL,
  VERIFY_TOOL,
} from './analysis-tools.ts';
import { mergeAnchors } from './anchors.ts';
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
  checkCategory,
  GateLedger,
  isAmbiguousAnchor,
  mergeCounters,
  normalizeComposed,
  normalizeGroups,
  normalizeReviewItems,
  startsInAny,
  startsInWindow,
} from './gates.ts';
import { checkPlan } from './plan-check.ts';
import { buildBackgroundQuery, renderSearchHits, searchBackground } from './search.ts';
import { createFindRange } from './text.ts';
import { planVerifyBatches } from './verify-batch.ts';
import { planWindows } from './windows.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { AnalysisStagePrompt } from '../../src/lib/domain/analysis-prompts.ts';
import type { Usage } from './analysis-llm.ts';
import type { ResolvedProfile } from './analysis-render.ts';
import type { Finding, Plan, Scene, Strength, WorkProfile } from './analysis-types.ts';
import type { GateRecord } from './gates.ts';
import type { AnalysisParams, FlowEnv } from './index.ts';

// 병렬 폭. 한 step 안에서 동시에 띄우는 호출 수이며, 그대로 subrequest 수가 된다.
const REVIEW_FANOUT = 12;
const VERIFY_FANOUT = 24;

// 짚을 곳 찾기 반복 횟수. 실측에서 세 회차가 서로 대등하게 겹쳤다 — 회차별 단독 발견이
// 위치 오차 100자 기준 16~23%, 300자 기준 1~10%로 특정 회차가 특별하지 않았다.
// 지적 수가 곧 중복 묶기·검증의 부하이므로 이 값은 뒤 단계 비용까지 함께 결정한다.
const REVIEW_RUNS = 2;

// 한 검증 호출이 판정할 지적 수의 상한. 원문을 공유해도 한 번에 너무 많이 물으면
// 뒤쪽 판정이 성의를 잃으므로 상한을 두고 같은 원문으로 호출을 나눈다.
//
// 진단용으로 1에 두었다. 묶어 물었을 때 통과율이 100%로 나온 원인이 배치인지 가리는 중이며,
// 판정이 끝나면 되돌리거나 안전한 값으로 다시 정해야 한다.
const VERIFY_BATCH = 1;

// 계획 검수 수렴 상한. approve가 나오면 조기 종료하고, 상한까지 미수렴이면 마지막 계획으로
// 진행한다 — 검수자가 만족을 모르는 경우 문서를 세우는 것보다 기록을 남기는 편이 낫다.
const PLAN_REVIEW_ROUNDS = 3;

// 검수는 빈 발견을 낼 수 있되 verdict는 강제된다 — 빈 배열이 공짜 탈출구가 아니라
// "발견을 지지할 수 없다"는 명시적 커밋이 되도록. approve만이 수렴 루프를 끝낸다.
const PLAN_REVIEW_SCHEMA = {
  name: 'plan_review',
  strict: true,
  schema: {
    type: 'object',
    properties: {
      findings: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            kind: { type: 'string', enum: ['over-protection', 'missing-axis', 'biased-axis', 'weak-axis', 'contract-violation'] },
            target: { type: 'string' },
            rationale: { type: 'string' },
            fix: { type: 'string' },
            // 무조건 필드다 — 장르 겸손을 산문으로 두면 안 묶인다. 실측: 검수자가 장르
            // 문법(가이드≈일반인)을 모른 채 텍스트 내적 모순을 0.97로 주입해 라운드 3에서
            // 사람이 기각한 지적이 부활했다.
            genreCheck: { type: 'string' },
            confidence: { type: 'number' },
          },
          required: ['kind', 'target', 'rationale', 'fix', 'genreCheck', 'confidence'],
          additionalProperties: false,
        },
      },
      verdict: { type: 'string', enum: ['approve', 'needs-attention'] },
    },
    required: ['findings', 'verdict'],
    additionalProperties: false,
  },
} as const;

type PlanReviewFinding = { kind: string; target: string; rationale: string; fix: string; genreCheck: string; confidence: number };
type PlanReview = { verdict: 'approve' | 'needs-attention'; findings: PlanReviewFinding[] };

// 계획 검수는 다른 벤더의 호출이다. GPT-5.6은 chat/completions에서 함수 도구와 추론을
// 함께 못 쓰므로 structured output으로 받는다 — RulingWorkflow와 같은 경로다.
const callPlanReview = async (
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
    response_format: { type: 'json_schema', json_schema: PLAN_REVIEW_SCHEMA as never },
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

export class AnalysisWorkflow extends WorkflowEntrypoint<FlowEnv, AnalysisParams> {
  async run(event: WorkflowEvent<AnalysisParams>, step: WorkflowStep) {
    const { runId, promptSetId, documentId, variantLabel } = event.payload;
    const db = createDb(this.env.DB);
    // 게이트웨이의 Anthropic 전용 엔드포인트. Unified Billing이라 Anthropic 키 없이
    // Cloudflare 토큰만으로 통과한다 — x-api-key 대신 Authorization: Bearer로 나간다.
    const client = new Anthropic({ apiKey: null, authToken: this.env.CLOUDFLARE_API_KEY, baseURL: this.env.CLOUDFLARE_AIGATEWAY_URL });

    const resolved = await step.do('resolve', async () => {
      const [doc] = await db.select().from(Documents).where(eq(Documents.id, documentId));
      if (!doc) throw new Error('document not found');
      const [set] = await db.select().from(AnalysisPromptSets).where(eq(AnalysisPromptSets.id, promptSetId));
      if (!set) throw new Error('prompt set not found');

      await db
        .update(PipelineRunDocs)
        .set({ status: 'running', phase: 'survey', error: null })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));

      return { content: doc.content, prompts: set.content };
    });

    const { content, prompts } = resolved;

    try {
      // ── SURVEY ── 전문 1회 통독. 위치는 인용문으로 받아 코드가 찾는다.
      const survey = await step.do('survey', LLM_STEP, async () => {
        return cachedStep(db, runId, documentId, 'survey', async () => {
          const usage = emptyUsage();
          const raw = await callTool<
            ResolvedProfile & {
              scenes: (Omit<Scene, 'start' | 'end'> & { startQuote: string; endQuote: string })[];
              nonAnalyticRanges: { startQuote: string; endQuote: string; reason: string }[];
            }
          >(client, prompts.survey, SURVEY_TOOL, `<원고>\n${content}\n</원고>`, usage);

          const findRange = createFindRange(content);
          let cursor = 0;
          const scenes: Scene[] = [];
          for (const scene of raw.scenes) {
            const range = findRange(scene.startQuote, scene.endQuote, cursor);
            if (!range) continue;
            cursor = range.rangeEnd;
            scenes.push({ ...scene, start: range.rangeStart, end: range.rangeEnd });
          }

          let nonCursor = 0;
          const nonAnalyticRanges: WorkProfile['nonAnalyticRanges'] = [];
          for (const item of raw.nonAnalyticRanges) {
            const range = findRange(item.startQuote, item.endQuote, nonCursor);
            if (!range) continue;
            nonCursor = range.rangeEnd;
            nonAnalyticRanges.push({ start: range.rangeStart, end: range.rangeEnd, reason: item.reason });
          }

          await addUsage(db, runId, documentId, 'survey', usage);
          return { profile: { ...raw, nonAnalyticRanges }, scenes };
        });
      });

      const profile = survey.profile as ResolvedProfile;
      const scenes = survey.scenes;
      // ── BACKGROUND ── 원작을 검색해 2차 창작 여부를 판정하고 배경을 만든다.
      //
      // 2차 창작 여부를 모델의 단일 판단에 맡기지 않는다 — 같은 문서가 실행에 따라 아니오로
      // 흔들려 배경이 통째로 빠진 일이 실제로 있었다(특정 2차창작 문서). survey의 값은 검색어
      // 힌트일 뿐이고, 검색이 무조건 돌며 원작이 특정되는지가 최종 판정이다.
      // 프롬프트가 없는 세트(기존 세트)는 이 단계를 통째로 건너뛴다.
      const backgroundPrompt = prompts.background;
      const background = backgroundPrompt
        ? await step.do('background', LLM_STEP, async () =>
            cachedStep(db, runId, documentId, 'background', async (): Promise<{ sourceName: string; brief: string } | null> => {
              const query = buildBackgroundQuery({ derivativeSource: profile.derivativeSource, properNouns: profile.properNouns });
              if (!query || !this.env.EXA_API_KEY) return null;
              try {
                const hits = await searchBackground({ apiKey: this.env.EXA_API_KEY, query });
                if (hits.length === 0) return null;
                const usage = emptyUsage();
                // 검색 결과를 그대로 넣지 않는다. 원문을 읽는 데 필요한 만큼만 추리게 한다.
                const result = await callTool<{ sourceIdentified: boolean; sourceName: string; brief: string; genreVariant: string }>(
                  client,
                  backgroundPrompt,
                  BACKGROUND_TOOL,
                  renderBackgroundInput(query, renderSearchHits(hits)),
                  usage,
                );
                if (!result.sourceIdentified || !result.brief.trim()) {
                  await addUsage(db, runId, documentId, 'background', usage);
                  return null;
                }

                // 장르 변형(AU)이 감지되면 그 장르의 문법을 따로 검색한다. 원작 검색 결과에는
                // 인물·줄거리만 있고 장르 문법(분류·관계 관습)이 없어, 검수자가 장르 독자에게
                // 자명한 것을 모순으로 주입한 일이 실제로 있었다.
                let brief = result.brief.trim();
                const genrePrompt = prompts.genre;
                const variant = result.genreVariant.trim();
                if (genrePrompt && variant) {
                  try {
                    const genreHits = await searchBackground({
                      apiKey: this.env.EXA_API_KEY,
                      query: `${variant} 세계관 설정 관습 용어 뜻`,
                    });
                    if (genreHits.length > 0) {
                      const { conventions } = await callTool<{ conventions: string }>(
                        client,
                        genrePrompt,
                        GENRE_TOOL,
                        renderBackgroundInput(variant, renderSearchHits(genreHits)),
                        usage,
                      );
                      if (conventions.trim()) brief += `\n\n[장르 관습 — ${variant}]\n${conventions.trim()}`;
                    }
                  } catch (err) {
                    console.warn(`genre search failed(무시): ${String(err).slice(0, 200)}`);
                  }
                }

                await addUsage(db, runId, documentId, 'background', usage);
                return { sourceName: result.sourceName.trim(), brief };
              } catch (err) {
                // 배경은 있으면 좋은 것이다. 검색이 막혔다고 분석을 세우지 않는다.
                console.error(`background failed: ${String(err)}`);
                return null;
              }
            }),
          )
        : null;

      // 검색 판정이 프로필의 짐작을 덮어쓴다. 원작이 특정되면 2차 창작이고, 특정되지 않으면
      // 오리지널로 취급한다 — 하류의 모든 판단이 이 한 줄을 전제로 삼는다. 배경 단계가 없는
      // 구 세트는 검색이 없으므로 프로필 값을 그대로 쓴다.
      const effectiveProfile: ResolvedProfile = backgroundPrompt
        ? background
          ? { ...profile, isDerivative: true, derivativeSource: background.sourceName || profile.derivativeSource || undefined }
          : { ...profile, isDerivative: false, derivativeSource: undefined }
        : profile;

      const conventionsBase = renderConventions(renderProfile(effectiveProfile, background?.brief ?? null));

      // ── PLAN ── 문서 수준 비평 계획. 무엇을 볼지(축)와 무엇을 건드리면 안 되는지(보호 기법)를
      // 생성 앞에서 확정한다. 프롬프트가 없는 세트는 기존 무계획 경로 그대로 돈다.
      //
      // 계획은 수렴 루프를 거친다: 코드 검증(checkPlan) → 교차 벤더 검수 → 수정, approve가
      // 나올 때까지 최대 PLAN_REVIEW_ROUNDS회. 루프의 호출 하나가 워크플로 스텝 하나다 —
      // 한 스텝에 뭉치면 진행이 안 보이고, 검수 하나가 죽었을 때 루프 전체가 재청구된다.
      // checkPlan은 순수 함수라 리플레이마다 본문에서 다시 계산해도 같은 값이 나온다.
      let plan: Plan | null = null;
      const planPrompt = prompts.plan;
      if (planPrompt) {
        await step.do('phase-plan', () => setPhase(db, runId, documentId, 'plan'));
        const manuscript = `<원고>\n${content}\n</원고>`;
        const planOptions = { conventions: conventionsBase };

        // 초안. 축 수 계약 위반이면 같은 스텝 안에서 한 번 반려·재수령한다 — 반려 판정이
        // 코드라 이 둘은 항상 함께 다시 돈다.
        const draft = await step.do('plan-draft', LLM_STEP, async () => {
          const { value, usage } = await cachedCall<Plan>(db, runId, documentId, 'plan/draft', async (usage) => {
            const first = await callTool<Plan>(client, planPrompt, PLAN_TOOL, manuscript, usage, planOptions);
            const check = checkPlan(content, first);
            if (check.axisCountOk) return first;
            return callTool<Plan>(
              client,
              planPrompt,
              PLAN_TOOL,
              [
                manuscript,
                '',
                `이전 계획이 반려되었습니다.\n${check.notes.map((n) => `- ${n}`).join('\n')}\n계약을 지켜 계획을 다시 세우세요.`,
              ].join('\n'),
              usage,
              planOptions,
            );
          });
          await addUsage(db, runId, documentId, 'plan', usage);
          return value;
        });

        let check = checkPlan(content, draft);
        let current = check.plan;

        const reviewPrompt = prompts.planReview;
        const rounds: { notes: string[]; review: PlanReview }[] = [];
        if (reviewPrompt) {
          const reviewSystem = [reviewPrompt.system, '', conventionsBase, '', manuscript].join('\n');
          for (let round = 0; round < PLAN_REVIEW_ROUNDS; round++) {
            const reviewInput = renderPlanReviewInput(current, check.notes);
            // 검수 실패를 삼키지 않는다 — 조용히 무검수로 진행하면 크레딧 소진 같은 계통
            // 장애가 결과 오염으로만 나타난다. 스텝 재시도가 이 호출 하나만 다시 문다.
            const review = await step.do(`plan-review-${round}`, LLM_STEP, async () => {
              const { value, usage } = await cachedCall<PlanReview>(db, runId, documentId, `plan/review/${round}`, (usage) =>
                callPlanReview(this.env, reviewPrompt, reviewSystem, reviewInput, usage),
              );
              await addUsage(db, runId, documentId, 'plan', usage);
              return value;
            });
            rounds.push({ notes: check.notes, review });
            if (review.verdict === 'approve') break;
            if (round === PLAN_REVIEW_ROUNDS - 1) {
              console.warn(`계획 검수 미수렴(${PLAN_REVIEW_ROUNDS}회) — 마지막 계획으로 진행`);
              break;
            }

            const reviseInput = [manuscript, '', renderPlanReviseInput(current, review.findings)].join('\n');
            const revised = await step.do(`plan-revise-${round}`, LLM_STEP, async () => {
              const { value, usage } = await cachedCall<Plan>(db, runId, documentId, `plan/revise/${round}`, (usage) =>
                callTool<Plan>(client, planPrompt, PLAN_TOOL, reviseInput, usage, planOptions),
              );
              await addUsage(db, runId, documentId, 'plan', usage);
              return value;
            });
            check = checkPlan(content, revised);
            current = check.plan;
          }
        }

        plan = current;
        // 최종 계획과 라운드 기록을 한 키로 남긴다 — 진단이 회차별 캐시를 뒤질 필요 없도록.
        const summary = { final: current, rounds, notes: check.notes };
        await step.do('plan-summary', async () => {
          await writeStageCache(db, cacheKey(runId, documentId, 'plan'), summary);
        });
      }

      // 이 문서의 모든 호출이 공유하는 앞머리. 프롬프트 캐싱의 대상이다. 계획도 규약처럼
      // 문서의 모든 검토 호출이 공유하므로 같은 층에 싣는다.
      const conventions = plan ? [conventionsBase, '', renderPlan(plan)].join('\n') : conventionsBase;
      const reviewTool = plan ? reviewToolWithAxes(plan.axes.map((a) => a.label)) : REVIEW_TOOL;
      const windows = planWindows(content, scenes);

      // ── REVIEW ── 창 × REVIEW_RUNS회를 전부 병렬로 띄운다.
      //
      // 지적이 스텝을 빠져나가기 전에 코드가 원문과 대조한다. 앵커가 찾히지 않으면 버리는 것은
      // 원래부터 있던 유일한 결정적 판정이었고, 나머지 — 창 밖 시작, 분석 대상 아닌 구간, 근거
      // 인용 — 는 지시문에만 있어 어겨져도 아무 신호가 없었다.
      await step.do('phase-review', () => setPhase(db, runId, documentId, 'review'));
      const jobs = windows.flatMap((window) => Array.from({ length: REVIEW_RUNS }, (_, run) => ({ window, run })));
      const findings: Finding[] = [];
      const strengths: Strength[] = [];
      const gateRecords: GateRecord[] = [];
      const counters: Record<string, number>[] = [];

      for (let i = 0; i < jobs.length; i += REVIEW_FANOUT) {
        const slice = jobs.slice(i, i + REVIEW_FANOUT);
        const batch = await step.do(`review-${i}`, LLM_STEP, async () => {
          const findRange = createFindRange(content);
          // 팬아웃 전에 접두부를 얹는다. 이게 없으면 동시에 나간 호출들이 서로의 쓰기를 보지
          // 못해 전원이 미스가 되고, 쓰기 프리미엄만 호출 수만큼 물게 된다.
          const warm = emptyUsage();
          await warmPrefix(client, prompts.review, reviewTool, { conventions, cache: true }, warm);

          // 원장은 호출 단위 캐시 안에서 만든다. 바깥에 두면 캐시가 적중한 호출의 판정이
          // 통째로 비어 버린다 — usage를 캐시에 함께 넣는 것과 같은 이유다.
          const results = await Promise.all(
            slice.map(({ window, run }) =>
              cachedCall(db, runId, documentId, `review/${window.index}/${run}`, async (usage) => {
                const ledger = new GateLedger();
                const anchorOf = (quoteStart: string, quoteEnd: string, label: string, payload: unknown) => {
                  const range = findRange(quoteStart, quoteEnd, window.start);
                  if (!range) return ledger.reject('anchor-unresolved', `${label}: ${quoteStart.slice(0, 30)}`, payload);
                  const span = { start: range.rangeStart, end: range.rangeEnd };
                  if (!startsInWindow(span, window)) {
                    return ledger.reject('anchor-out-of-window', `${label}: ${quoteStart.slice(0, 30)}`, payload);
                  }
                  if (startsInAny(span, profile.nonAnalyticRanges)) {
                    return ledger.reject('anchor-non-analytic', `${label}: ${quoteStart.slice(0, 30)}`, payload);
                  }
                  if (isAmbiguousAnchor(window.text, quoteStart)) {
                    ledger.note('anchor-ambiguous', `${label}: ${quoteStart.slice(0, 30)}`, payload);
                  }
                  return span;
                };

                type Raw = {
                  findings: Omit<Finding, 'matchStart' | 'matchEnd' | 'runIndex' | 'stumbleStart' | 'stumbleEnd'>[];
                  strengths: Omit<Strength, 'matchStart' | 'matchEnd'>[];
                };
                const raw = await callTool<Raw>(client, prompts.review, reviewTool, renderReviewInput(scenes, window), usage, {
                  conventions,
                  cache: true,
                });

                const kept: Finding[] = [];
                for (const f of raw.findings) {
                  ledger.count('finding.emitted');

                  const span = anchorOf(f.quoteStart, f.quoteEnd, '지적', f);
                  if (span === false) continue;

                  // 멈춘 자리는 조건 없이 모든 지적에 요구한다. 갈래에 따라 면제하면 면제받는
                  // 쪽으로 몰린다 — 앞선 판에서 실제로 그랬다.
                  const found = findRange(f.stumbleQuote, f.stumbleQuote, 0);
                  if (!found) {
                    ledger.reject('stumble-unresolved', f.stumbleQuote.slice(0, 30), f);
                    continue;
                  }
                  const stumble = { start: found.rangeStart, end: found.rangeEnd };
                  if (!startsInWindow(stumble, window)) {
                    ledger.reject('stumble-out-of-window', f.stumbleQuote.slice(0, 30), f);
                    continue;
                  }

                  ledger.count('finding.kept');
                  kept.push({
                    ...f,
                    matchStart: span.start,
                    matchEnd: span.end,
                    stumbleStart: stumble.start,
                    stumbleEnd: stumble.end,
                    runIndex: run,
                  });
                }

                const keptStrengths: Strength[] = [];
                for (const s of raw.strengths) {
                  ledger.count('strength.emitted');
                  const span = anchorOf(s.quoteStart, s.quoteEnd, '강점', s);
                  if (span === false) continue;
                  ledger.count('strength.kept');
                  keptStrengths.push({ ...s, matchStart: span.start, matchEnd: span.end });
                }

                return { findings: kept, strengths: keptStrengths, gates: ledger.records, counters: ledger.counters };
              }),
            ),
          );
          await addUsage(db, runId, documentId, 'review', sumUsage([warm, ...results.map((r) => r.usage)]));
          return {
            findings: results.flatMap((r) => r.value.findings),
            strengths: results.flatMap((r) => r.value.strengths),
            gates: results.flatMap((r) => r.value.gates),
            counters: mergeCounters(results.map((r) => r.value.counters)),
          };
        });
        findings.push(...batch.findings);
        strengths.push(...batch.strengths);
        gateRecords.push(...batch.gates);
        counters.push(batch.counters);
      }

      // ── DEDUPE ── 같은 문제끼리 묶는다. 위치로 후보를 좁히지 않는다 —
      // 반복 유형 묶기는 위치가 겹치지 않는 경우가 본질이다.
      await step.do('phase-dedupe', () => setPhase(db, runId, documentId, 'dedupe'));
      const deduped = await step.do('dedupe', LLM_STEP, async () => {
        const usage = emptyUsage();
        const ledger = new GateLedger();
        const { groups: raw } = await callTool<{ groups: { members: number[]; representative: number; reason: string }[] }>(
          client,
          prompts.dedupe,
          DEDUPE_TOOL,
          renderDedupeInput(findings),
          usage,
        );
        await addUsage(db, runId, documentId, 'dedupe', usage);

        // 배정되지 않은 지적은 혼자인 묶음으로 살리고, 두 묶음에 든 지적은 뒤엣것을 버린다.
        // 앞은 원래 있었고 뒤는 없어서 같은 지적이 피드백 두 건이 되어 나갔다.
        ledger.count('finding.in', findings.length);
        const normalized = normalizeGroups(raw, findings.length, ledger);
        ledger.count('group.out', normalized.length);
        return { groups: normalized, gates: ledger.records, counters: ledger.counters };
      });
      const groups = deduped.groups;
      gateRecords.push(...deduped.gates);
      counters.push(deduped.counters);

      // ── VERIFY ── 판정은 지적마다 독립이다. 서로 비교하게 두면 상대적 중요도가 섞인다.
      //
      // 프롬프트 세트에 verify가 없으면 이 단계를 통째로 건너뛰고 근거 확인을 피드백 쓰기에
      // 맡긴다. 검증은 지적마다 원문을 다시 보내느라 비용의 39%를 쓰는데 실제로 걷어낸 건
      // 8%였다 — 그 확인이 한 번 도는 단계 안에서 되는지 재보려는 것이다.
      type VerifiedAnchor = {
        quoteStart: string;
        quoteEnd: string;
        matchStart: number | null;
        matchEnd: number | null;
        ground: string;
        reason: string;
      };

      // REVIEW가 같은 대목을 조금씩 다른 인용으로 잡으므로 여기서 한 대목으로 합친다.
      const groupAnchors = groups.map((group) => {
        const seen = new Map<string, { quoteStart: string; quoteEnd: string; matchStart: number | null; matchEnd: number | null }>();
        for (const m of group.members) {
          const f = findings[m];
          if (f) {
            seen.set(String(f.matchStart), {
              quoteStart: f.quoteStart,
              quoteEnd: f.quoteEnd,
              matchStart: f.matchStart,
              matchEnd: f.matchEnd,
            });
          }
        }
        return mergeAnchors([...seen.values()]);
      });

      const verifyPrompt = prompts.verify;
      const verified: { representative: number; anchors: VerifiedAnchor[] }[] = [];

      if (verifyPrompt) {
        await step.do('phase-verify', () => setPhase(db, runId, documentId, 'verify'));

        const plans = planVerifyBatches(
          scenes,
          groupAnchors.map((anchors) => ({ anchors })),
          VERIFY_BATCH,
        );

        // 캐시 키를 순번이 아니라 묶음의 내용으로 짓는다. 워크플로 인스턴스가 통째로 다시 뜨면
        // 중복 묶기가 다시 돌아 묶음 구성이 달라질 수 있는데, 순번으로 키를 만들면 그때 엉뚱한
        // 판정이 재사용된다. 구성이 바뀌면 키도 바뀌어 자연히 다시 돈다.
        const planKey = (plan: (typeof plans)[number]) => `verify/${plan.sceneIndexes?.join('.') ?? 'full'}/${plan.items.join('-')}`;

        const runPlan = (plan: (typeof plans)[number]) =>
          cachedCall(db, runId, documentId, planKey(plan), async (usage) => {
            const items = plan.items.map((index) => ({ finding: findings[groups[index].representative], anchors: groupAnchors[index] }));
            const excerpt = renderScenes(content, scenes, plan.sceneIndexes);

            let parsed: { findings: { index: number; anchors: { index: number; ground: string; reason: string }[] }[] } = { findings: [] };
            try {
              parsed = await callTool<typeof parsed>(
                client,
                verifyPrompt,
                VERIFY_TOOL,
                renderVerifyInput(excerpt, plan.sceneIndexes === null, items),
                usage,
                { conventions, cache: true },
              );
            } catch {
              // 검증 실패는 통과로 둔다. 검증기 오류로 지적이 조용히 사라지면 추적할 수 없다.
            }

            return plan.items.map((index, itemIndex) => {
              const verdict = parsed.findings.find((f) => f.index === itemIndex);
              return {
                representative: groups[index].representative,
                anchors: groupAnchors[index].map((a, k) => {
                  const v = verdict?.anchors.find((x) => x.index === k);
                  return { ...a, ground: v?.ground ?? 'valid', reason: v?.reason ?? '판정 없음 — 통과 처리' };
                }),
              };
            });
          });

        for (let i = 0; i < plans.length; i += VERIFY_FANOUT) {
          const slice = plans.slice(i, i + VERIFY_FANOUT);
          const batch = await step.do(`verify-${i}`, LLM_STEP, async () => {
            if (slice.length === 0) return [];
            // 예열로 접두부를 얹은 뒤 전부 병렬로 띄운다.
            const warm = emptyUsage();
            await warmPrefix(client, verifyPrompt, VERIFY_TOOL, { conventions, cache: true }, warm);
            const results = await Promise.all(slice.map((plan) => runPlan(plan)));
            await addUsage(db, runId, documentId, 'verify', sumUsage([warm, ...results.map((r) => r.usage)]));
            return results.flatMap((r) => r.value);
          });
          verified.push(...batch);
        }
      } else {
        // 검증을 건너뛴다 — 모든 묶음을 그대로 피드백 쓰기로 넘긴다.
        verified.push(
          ...groups.map((group, index) => ({
            representative: group.representative,
            anchors: groupAnchors[index].map((a) => ({ ...a, ground: 'valid', reason: '검증 단계 없음' })),
          })),
        );
      }

      // ── COMPOSE ── 피드백을 먼저 확정하고 그 집합으로 총평을 쓴다.
      await step.do('phase-compose', () => setPhase(db, runId, documentId, 'compose'));
      const survived = verified
        .map((g) => ({ ...g, anchors: g.anchors.filter((a) => a.ground === 'valid') }))
        .filter((g) => g.anchors.length > 0)
        .toSorted((a, b) => (a.anchors[0]?.matchStart ?? 0) - (b.anchors[0]?.matchStart ?? 0));

      const composedStep = await step.do('compose', LLM_STEP, async () => {
        const usage = emptyUsage();
        const ledger = new GateLedger();
        type Composed = { feedbacks: { groupIndex: number; category: string; body: string }[] };
        const { feedbacks } = await callTool<Composed>(
          client,
          prompts.compose,
          COMPOSE_TOOL,
          renderComposeInput(survived, findings),
          usage,
          { conventions, cache: false },
        );
        await addUsage(db, runId, documentId, 'compose', usage);

        // "주어진 것을 모두 옮긴다"가 계약인데 아무도 세지 않았다. 빠뜨리면 지적이 조용히
        // 사라지고, 없는 번호를 내면 앵커 없는 빈 피드백이 저장됐다.
        ledger.count('group.in', survived.length);
        ledger.count('feedback.emitted', feedbacks.length);
        const kept = normalizeComposed(feedbacks, survived.length, ledger);
        ledger.count('feedback.kept', kept.length);
        return {
          feedbacks: kept.map((f) => ({
            category: checkCategory(f.category, ledger),
            polarity: 'issue' as const,
            body: f.body,
            anchors: survived[f.groupIndex].anchors.map((a) => ({
              quoteStart: a.quoteStart,
              quoteEnd: a.quoteEnd,
              matchStart: a.matchStart,
              matchEnd: a.matchEnd,
            })),
          })),
          gates: ledger.records,
          counters: ledger.counters,
        };
      });
      const composed = composedStep.feedbacks;
      gateRecords.push(...composedStep.gates);
      counters.push(composedStep.counters);

      // ── SELFCHECK ── 완성된 피드백이 원고에 근거를 두는지 하나씩 확인한다.
      //
      // 한 건씩 묻는 것이 핵심이다. 여러 건을 묶으면(배치 8) 기각이 0건이 됐고, 쓰기와 같은
      // 호출에 섞어도(피드백 쓰기 흡수) 0건이었다. 판정은 독립된 단일 물음일 때만 작동한다.
      // 원고를 캐시 접두부에 두어 첫 호출만 쓰고 나머지는 10분의 1 값으로 읽는다.
      const selfcheckPrompt = prompts.selfcheck;
      let delivered = composed;
      if (selfcheckPrompt && composed.length > 0) {
        await step.do('phase-selfcheck', () => setPhase(db, runId, documentId, 'selfcheck'));
        const options = { conventions, manuscript: content, cache: true };

        const verdicts = await step.do('selfcheck', LLM_STEP, async () => {
          const warm = emptyUsage();
          await warmPrefix(client, selfcheckPrompt, SELFCHECK_TOOL, options, warm);
          const results = await Promise.all(
            composed.map((feedback, i) =>
              cachedCall(db, runId, documentId, `selfcheck/${i}`, async (usage) => {
                try {
                  return await callTool<{ grounded: boolean; reason: string }>(
                    client,
                    selfcheckPrompt,
                    SELFCHECK_TOOL,
                    renderSelfCheckInput(feedback, feedback.anchors),
                    usage,
                    options,
                  );
                } catch {
                  // 확인에 실패하면 통과로 둔다. 검사기 오류로 피드백이 조용히 사라지면 안 된다.
                  return { grounded: true, reason: '판정 없음 — 통과 처리' };
                }
              }),
            ),
          );
          await addUsage(db, runId, documentId, 'selfcheck', sumUsage([warm, ...results.map((r) => r.usage)]));
          return results.map((r) => r.value);
        });

        const dropped = verdicts.map((v, i) => ({ index: i, ...v })).filter((v) => !v.grounded);
        console.warn(`selfcheck: 피드백 ${composed.length}건 중 ${dropped.length}건 탈락`);
        for (const d of dropped) console.warn(`  [탈락 ${d.index}] ${d.reason.slice(0, 200)}`);
        await writeStageCache(db, cacheKey(runId, documentId, 'selfcheck/dropped'), dropped);
        delivered = composed.filter((_, i) => verdicts[i]?.grounded !== false);
      }

      // 강점 후보는 손대지 않고 전부 총평으로 넘긴다.
      //
      // 지적과 위치가 겹치는 강점을 걷어내고 회차 간 중복을 합치는 코드가 있었으나 버렸다.
      // 위치 겹침은 모순이 아니다 — "이 대목은 작동하는데 그 안의 이 한 줄이 걸린다"는 편집에서
      // 정상이며, 실측에서 겹침 판정은 한 번도 발화하지 않았다. 무엇이 모순이고 어느 중복이 더
      // 잘 쓰였는지는 확정된 지적과 후보를 함께 보는 총평이 판단한다.
      const offeredStrengths = strengths;

      // 총평은 JSON 문자열로 넘긴다 — step 반환값 타입이 Serializable로 좁혀져 있어
      // 임의 객체를 그대로 통과시키지 못한다.
      const reviewStep = await step.do('compose-review', LLM_STEP, async () => {
        const usage = emptyUsage();
        const ledger = new GateLedger();
        type Raw = {
          characterization: string;
          strengths: { body: string; quoteStart: string; quoteEnd: string }[];
          patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
          priority: { body: string; feedbackIndexes: number[] }[];
        };
        const raw = await callTool<Raw>(
          client,
          prompts.composeReview,
          COMPOSE_REVIEW_TOOL,
          renderComposeReviewInput(effectiveProfile, delivered, offeredStrengths),
          usage,
        );
        await addUsage(db, runId, documentId, 'composeReview', usage);

        // 총평이 고른 강점을 원문 위치로 되돌린다. 후보로 준 인용을 그대로 옮겼으면 이미
        // 잡아둔 위치를 쓰고, 손댔으면 원고에서 다시 찾는다 — 위치가 없으면 화면이 그 대목을
        // 가리키지 못하고 강점은 다시 위치 없는 감상이 된다.
        const offered = new Map(offeredStrengths.map((s) => [`${s.quoteStart}|${s.quoteEnd}`, s]));
        const findRange = createFindRange(content);
        const resolvedStrengths = (raw.strengths ?? []).map((s) => {
          const hit = offered.get(`${s.quoteStart}|${s.quoteEnd}`);
          if (hit) return { ...s, matchStart: hit.matchStart, matchEnd: hit.matchEnd };
          const range = findRange(s.quoteStart, s.quoteEnd, 0);
          if (!range) ledger.note('review-strength-unresolved', s.quoteStart.slice(0, 30));
          return { ...s, matchStart: range?.rangeStart ?? null, matchEnd: range?.rangeEnd ?? null };
        });

        // 총평이 없는 피드백 번호를 가리켜도 그대로 저장돼 왔다. 화면은 이미 버리고 있었지만,
        // 걸러낸 자리에서 세어야 총평이 얼마나 헛짚는지 보인다.
        const review = {
          ...raw,
          strengths: resolvedStrengths,
          patterns: normalizeReviewItems(raw.patterns ?? [], delivered.length, ledger),
          priority: normalizeReviewItems(raw.priority ?? [], delivered.length, ledger),
        };
        ledger.count('strength.chosen', resolvedStrengths.length);
        return { review: JSON.stringify(review), gates: ledger.records, counters: ledger.counters };
      });
      const review = JSON.parse(reviewStep.review) as Record<string, unknown>;
      gateRecords.push(...reviewStep.gates);
      counters.push(reviewStep.counters);

      // 무엇이 왜 걸러졌는지 남긴다. 기록이 없으면 게이트가 과한지 모자란지 다음 라운드에서
      // 판단할 근거가 없다 — 지금까지 조용히 지나간 것들이 정확히 그래서 안 보였다.
      await step.do('gates', async () => {
        const counts: Record<string, number> = {};
        for (const r of gateRecords) counts[`${r.action}:${r.gate}`] = (counts[`${r.action}:${r.gate}`] ?? 0) + 1;
        const totals = mergeCounters(counters);
        await writeStageCache(db, cacheKey(runId, documentId, 'gates'), { counts, totals, records: gateRecords });
        console.warn(`gates: ${JSON.stringify(counts)} / totals: ${JSON.stringify(totals)}`);
      });

      // ── 저장 ──
      await step.do('persist', async () => {
        const [existing] = await db
          .select({ id: FeedbackSets.id })
          .from(FeedbackSets)
          .where(and(eq(FeedbackSets.runId, runId), eq(FeedbackSets.documentId, documentId)));
        const setId = existing?.id ?? nanoid();
        if (existing) {
          await db.update(FeedbackSets).set({ review }).where(eq(FeedbackSets.id, setId));
        } else {
          await db.insert(FeedbackSets).values({ id: setId, runId, documentId, variantId: variantLabel, review });
        }

        for (const [ord, feedback] of delivered.entries()) {
          const feedbackId = nanoid();
          const head = feedback.anchors[0];
          await db.insert(Feedbacks).values({
            id: feedbackId,
            setId,
            ord,
            // 구 파이프라인이 쓰는 컬럼이라 NOT NULL이다. 첫 앵커를 넣고 전체는 앵커 테이블에 둔다.
            startText: head?.quoteStart ?? '',
            endText: head?.quoteEnd ?? '',
            matchStart: head?.matchStart ?? null,
            matchEnd: head?.matchEnd ?? null,
            category: feedback.category,
            polarity: feedback.polarity,
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

        await db
          .update(PipelineRunDocs)
          .set({ status: 'done', phase: 'done', doneChunks: delivered.length, totalChunks: delivered.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { feedbacks: delivered.length };
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
