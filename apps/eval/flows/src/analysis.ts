import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, eq, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import {
  relevantText,
  renderComposeInput,
  renderComposeReviewInput,
  renderDedupeInput,
  renderProfile,
  renderReviewInput,
  renderVerifyInput,
} from './analysis-render.ts';
import { COMPOSE_REVIEW_TOOL, COMPOSE_TOOL, DEDUPE_TOOL, REVIEW_TOOL, SURVEY_TOOL, VERIFY_TOOL } from './analysis-tools.ts';
import { mergeAnchors } from './anchors.ts';
import {
  AnalysisPromptSets,
  createDb,
  Documents,
  FeedbackAnchors,
  Feedbacks,
  FeedbackSets,
  PipelineRunDocs,
  PipelineRuns,
  readStageCache,
  writeStageCache,
} from './db.ts';
import { createOpenAI } from './llm.ts';
import { createFindRange } from './text.ts';
import { schemaViolations } from './tool-schema.ts';
import { planWindows } from './windows.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type OpenAI from 'openai';
import type { AnalysisStagePrompt } from '../../src/lib/domain/analysis-prompts.ts';
import type { ResolvedProfile } from './analysis-render.ts';
import type { Finding, Scene, WorkProfile } from './analysis-types.ts';
import type { Db } from './db.ts';
import type { AnalysisParams, FlowEnv } from './index.ts';

// 병렬 폭. 한 step 안에서 동시에 띄우는 호출 수이며, 그대로 subrequest 수가 된다.
const REVIEW_FANOUT = 12;
const VERIFY_FANOUT = 24;
const REVIEW_RUNS = 3;

// 문서 30편을 한꺼번에 띄우면 짚을 곳 찾기에서만 Opus 호출 90개가 동시에 나간다. 게이트웨이가
// 이를 늦추면서 개별 호출이 늘어지고, 실측에서 정상 문서도 문서당 8~17분이 걸렸다.
// 15분에서는 긴 문서 3편이 스텝 타임아웃으로 죽었으므로 여유를 두 배로 잡는다.
const LLM_STEP = { retries: { limit: 2, delay: '10 seconds' as const, backoff: 'exponential' as const }, timeout: '30 minutes' as const };

// cachedTokens는 promptTokens에 포함된 값이다(별도 합이 아니다). 캐시 읽기는 입력 단가의
// 10%라 비용을 낼 때 이 몫을 따로 떼어야 한다. 0으로 남으면 캐싱이 꺼져 있다는 근거가 된다.
type Usage = { promptTokens: number; completionTokens: number; cachedTokens: number };

const emptyUsage = (): Usage => ({ promptTokens: 0, completionTokens: 0, cachedTokens: 0 });

const addUsage = async (db: Db, runId: string, usage: Usage): Promise<void> => {
  if (usage.promptTokens === 0 && usage.completionTokens === 0) return;
  await db
    .update(PipelineRuns)
    .set({
      promptTokens: sql`${PipelineRuns.promptTokens} + ${Math.round(usage.promptTokens)}`,
      completionTokens: sql`${PipelineRuns.completionTokens} + ${Math.round(usage.completionTokens)}`,
      cachedTokens: sql`${PipelineRuns.cachedTokens} + ${Math.round(usage.cachedTokens)}`,
    })
    .where(eq(PipelineRuns.id, runId));
};

// 게이트웨이 캐시를 우회한다. REVIEW 3회가 의미를 가지려면 매번 모델을 다시 태워야 한다.
const SKIP_CACHE = { headers: { 'cf-aig-skip-cache': 'true' } };

const callTool = async <T>(
  openai: OpenAI,
  prompt: AnalysisStagePrompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  userContent: string,
  usage: Usage,
): Promise<T> => {
  const messages: OpenAI.Chat.Completions.ChatCompletionMessageParam[] = [
    { role: 'system', content: prompt.system },
    { role: 'user', content: userContent },
  ];
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: prompt.model,
    messages,
    tools: [tool],
    tool_choice: { type: 'function', function: { name: tool.function.name } },
  };
  if (prompt.effort) params.reasoning_effort = prompt.effort as never;
  if (prompt.temperature !== undefined && prompt.temperature !== null) params.temperature = prompt.temperature;

  // 스키마를 어긴 응답은 한 번만 지적해 다시 받는다. 스텝 재시도에 맡기면 같은 스텝의
  // 병렬 호출 수십 개가 통째로 다시 도는 반면, 여기서는 문제가 된 호출 하나만 다시 태운다.
  let violations: string[] = [];
  for (let attempt = 0; attempt < 2; attempt++) {
    const response = await openai.chat.completions.create({ ...params, messages }, SKIP_CACHE);
    usage.promptTokens += response.usage?.prompt_tokens ?? 0;
    usage.completionTokens += response.usage?.completion_tokens ?? 0;
    usage.cachedTokens += response.usage?.prompt_tokens_details?.cached_tokens ?? 0;

    const call = response.choices[0]?.message?.tool_calls?.[0];
    if (!call || !('function' in call)) throw new Error(`${tool.function.name}: 도구 호출 없음`);

    const parsed = JSON.parse(call.function.arguments) as T;
    violations = schemaViolations(tool.function.parameters, parsed);
    if (violations.length === 0) return parsed;

    messages.push(
      { role: 'assistant', tool_calls: [{ id: call.id, type: 'function', function: call.function }] },
      {
        role: 'tool',
        tool_call_id: call.id,
        content: `스키마를 어겼습니다.\n${violations.join('\n')}\n배열은 문자열로 감싸지 말고 배열 그대로 보내세요. 전체를 다시 채워 보내세요.`,
      },
    );
  }

  throw new Error(`${tool.function.name}: 스키마 위반 ${violations.join(' / ')}`);
};

// 재시도는 새 워크플로 인스턴스를 띄우므로 Cloudflare의 스텝 캐시가 통하지 않는다 — 중복 묶기에서
// 죽은 문서가 작품 파악과 짚을 곳 찾기를 통째로 다시 지불하게 된다. 값비싼 단계만 D1에 남겨
// 인스턴스가 바뀌어도 이어서 돌게 한다. 키에 runId가 있어 프롬프트가 바뀌면 자연히 무효가 된다.
const cacheKey = (runId: string, documentId: string, stage: string): string => `analysis/${runId}/${documentId}/${stage}`;

const cachedStep = async <T>(db: Db, runId: string, documentId: string, stage: string, compute: () => Promise<T>): Promise<T> => {
  const cached = await readStageCache<T>(db, cacheKey(runId, documentId, stage));
  if (cached !== null) return cached;
  const value = await compute();
  await writeStageCache(db, cacheKey(runId, documentId, stage), value);
  return value;
};

const setPhase = async (db: Db, runId: string, documentId: string, phase: string): Promise<void> => {
  await db
    .update(PipelineRunDocs)
    .set({ phase })
    .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
};

export class AnalysisWorkflow extends WorkflowEntrypoint<FlowEnv, AnalysisParams> {
  async run(event: WorkflowEvent<AnalysisParams>, step: WorkflowStep) {
    const { runId, promptSetId, documentId, variantLabel } = event.payload;
    const db = createDb(this.env.DB);
    const openai = createOpenAI(this.env.CLOUDFLARE_API_KEY, this.env.CLOUDFLARE_AIGATEWAY_URL);

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
          >(openai, prompts.survey, SURVEY_TOOL, `<원고>\n${content}\n</원고>`, usage);

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

          await addUsage(db, runId, usage);
          return { profile: { ...raw, nonAnalyticRanges }, scenes };
        });
      });

      const profile = survey.profile as ResolvedProfile;
      const scenes = survey.scenes;
      const profileText = renderProfile(profile);
      const windows = planWindows(content, scenes);

      // ── REVIEW ── 창 × 3회를 전부 병렬로 띄운다. 창이 하나뿐인 문서는 3회가 한 번에 나간다.
      await step.do('phase-review', () => setPhase(db, runId, documentId, 'review'));
      const jobs = windows.flatMap((window) => Array.from({ length: REVIEW_RUNS }, (_, run) => ({ window, run })));
      const findings: Finding[] = [];
      for (let i = 0; i < jobs.length; i += REVIEW_FANOUT) {
        const slice = jobs.slice(i, i + REVIEW_FANOUT);
        const batch = await step.do(`review-${i}`, LLM_STEP, async () => {
          return cachedStep(db, runId, documentId, `review/${i}`, async () => {
            const usage = emptyUsage();
            const findRange = createFindRange(content);
            const results = await Promise.all(
              slice.map(async ({ window, run }) => {
                const { findings: raw } = await callTool<{ findings: Omit<Finding, 'matchStart' | 'matchEnd' | 'runIndex'>[] }>(
                  openai,
                  prompts.review,
                  REVIEW_TOOL,
                  renderReviewInput(profileText, scenes, window),
                  usage,
                );
                return raw.map((f) => {
                  const range = findRange(f.quoteStart, f.quoteEnd, window.start);
                  return { ...f, matchStart: range?.rangeStart ?? null, matchEnd: range?.rangeEnd ?? null, runIndex: run };
                });
              }),
            );
            await addUsage(db, runId, usage);
            return results.flat();
          });
        });
        findings.push(...batch);
      }

      // ── DEDUPE ── 같은 문제끼리 묶는다. 위치로 후보를 좁히지 않는다 —
      // 반복 유형 묶기는 위치가 겹치지 않는 경우가 본질이다.
      await step.do('phase-dedupe', () => setPhase(db, runId, documentId, 'dedupe'));
      const groups = await step.do('dedupe', LLM_STEP, async () => {
        const usage = emptyUsage();
        const { groups: raw } = await callTool<{ groups: { members: number[]; representative: number; reason: string }[] }>(
          openai,
          prompts.dedupe,
          DEDUPE_TOOL,
          renderDedupeInput(findings),
          usage,
        );
        await addUsage(db, runId, usage);

        // 배정되지 않은 지적은 혼자인 묶음으로 살린다 — 묶기 실패로 지적이 사라지면 안 된다.
        const assigned = new Set(raw.flatMap((g) => g.members));
        const orphans = findings.map((_, i) => i).filter((i) => !assigned.has(i));
        return [...raw, ...orphans.map((i) => ({ members: [i], representative: i, reason: '' }))];
      });

      // ── VERIFY ── 묶음마다 독립 판정. 서로 비교하게 두면 상대적 중요도가 섞인다.
      await step.do('phase-verify', () => setPhase(db, runId, documentId, 'verify'));
      type VerifiedAnchor = {
        quoteStart: string;
        quoteEnd: string;
        matchStart: number | null;
        matchEnd: number | null;
        ground: string;
        reason: string;
      };
      const verified: { representative: number; anchors: VerifiedAnchor[] }[] = [];
      for (let i = 0; i < groups.length; i += VERIFY_FANOUT) {
        const slice = groups.slice(i, i + VERIFY_FANOUT);
        const batch = await step.do(`verify-${i}`, LLM_STEP, async () => {
          const usage = emptyUsage();
          const results = await Promise.all(
            slice.map(async (group) => {
              const rep = findings[group.representative];
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
              // REVIEW 3회가 같은 대목을 조금씩 다른 인용으로 잡으므로 여기서 한 대목으로 합친다.
              const anchors = mergeAnchors([...seen.values()]);
              const excerpt = relevantText(content, scenes, anchors);

              let parsed: { anchors: { index: number; ground: string; reason: string }[] } = { anchors: [] };
              try {
                parsed = await callTool<{ anchors: { index: number; ground: string; reason: string }[] }>(
                  openai,
                  prompts.verify,
                  VERIFY_TOOL,
                  renderVerifyInput(profileText, excerpt, excerpt.length === content.length, rep, anchors),
                  usage,
                );
              } catch {
                // 검증 실패는 통과로 둔다. 검증기 오류로 지적이 조용히 사라지면 추적할 수 없다.
              }

              return {
                representative: group.representative,
                anchors: anchors.map((a, k) => {
                  const v = parsed.anchors.find((x) => x.index === k);
                  return { ...a, ground: v?.ground ?? 'valid', reason: v?.reason ?? '판정 없음 — 통과 처리' };
                }),
              };
            }),
          );
          await addUsage(db, runId, usage);
          return results;
        });
        verified.push(...batch);
      }

      // ── COMPOSE ── 피드백을 먼저 확정하고 그 집합으로 총평을 쓴다.
      await step.do('phase-compose', () => setPhase(db, runId, documentId, 'compose'));
      const survived = verified
        .map((g) => ({ ...g, anchors: g.anchors.filter((a) => a.ground === 'valid') }))
        .filter((g) => g.anchors.length > 0)
        .toSorted((a, b) => (a.anchors[0]?.matchStart ?? 0) - (b.anchors[0]?.matchStart ?? 0));

      const composed = await step.do('compose', LLM_STEP, async () => {
        const usage = emptyUsage();
        const { feedbacks } = await callTool<{ feedbacks: { groupIndex: number; category: string; polarity: string; body: string }[] }>(
          openai,
          prompts.compose,
          COMPOSE_TOOL,
          renderComposeInput(survived, findings),
          usage,
        );
        await addUsage(db, runId, usage);
        return feedbacks.map((f) => ({
          category: f.category,
          polarity: f.polarity,
          body: f.body,
          anchors: (survived[f.groupIndex]?.anchors ?? []).map((a) => ({
            quoteStart: a.quoteStart,
            quoteEnd: a.quoteEnd,
            matchStart: a.matchStart,
            matchEnd: a.matchEnd,
          })),
        }));
      });

      // 총평은 JSON 문자열로 넘긴다 — step 반환값 타입이 Serializable로 좁혀져 있어
      // 임의 객체를 그대로 통과시키지 못한다.
      const reviewJson = await step.do('compose-review', LLM_STEP, async () => {
        const usage = emptyUsage();
        const raw = await callTool<Record<string, unknown>>(
          openai,
          prompts.composeReview,
          COMPOSE_REVIEW_TOOL,
          renderComposeReviewInput(profile, composed),
          usage,
        );
        await addUsage(db, runId, usage);
        return JSON.stringify(raw);
      });
      const review = JSON.parse(reviewJson) as Record<string, unknown>;

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

        for (const [ord, feedback] of composed.entries()) {
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
          .set({ status: 'done', phase: 'done', doneChunks: composed.length, totalChunks: composed.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { feedbacks: composed.length };
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
