import Anthropic from '@anthropic-ai/sdk';
import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, asc, eq, inArray } from 'drizzle-orm';
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
import { renderConventions, renderProfile, renderSelfCheckInput } from './analysis-render.ts';
import { DEFENSE_TOOL, SELFCHECK_TOOL, SURVEY_TOOL } from './analysis-tools.ts';
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
import { createFindRange } from './text.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { ResolvedProfile } from './analysis-render.ts';
import type { WorkProfile } from './analysis-types.ts';
import type { FlowEnv, JudgeParams } from './index.ts';

// 판정은 피드백마다 독립 호출이다 — 묶어 물으면 통과율이 100%가 된다는 것이 실측으로 남아
// 있다. 여기서 제한하는 것은 병렬 폭뿐이다.
const JUDGE_FANOUT = 16;

type JudgeFeedback = { id: string; ord: number; category: string; body: string; anchors: { quoteStart: string; quoteEnd: string }[] };

// 판정 실패를 통과로 위장하지 않는다 — 이 워크플로는 측정용이라, 오류를 통과로 채우면
// 보존율이 부풀어 기준 판정 자체가 오염된다.
type JudgeResult<T> = T | { error: string };
type Selfcheck = { grounded: boolean; reason: string };
type Defense = { defense: string; assessment: string; verdict: 'dismiss' | 'uphold' };

/**
 * 이미 저장된 실행의 피드백들에 판정 두 단계만 다시 돌린다.
 *
 * 라운드 3의 사람 판정(feedback_verdicts)이 라벨이 되므로, 생성 없이 판정 단계의
 * 기각률·보존율을 잴 수 있다. 결과는 stage_cache의 results 키에 피드백 id로 남아
 * 라벨과 직접 조인된다.
 */
export class JudgeWorkflow extends WorkflowEntrypoint<FlowEnv, JudgeParams> {
  async run(event: WorkflowEvent<JudgeParams>, step: WorkflowStep) {
    const { runId, promptSetId, sourceRunId, documentId } = event.payload;
    const db = createDb(this.env.DB);
    const client = new Anthropic({ apiKey: null, authToken: this.env.CLOUDFLARE_API_KEY, baseURL: this.env.CLOUDFLARE_AIGATEWAY_URL });

    const resolved = await step.do('resolve', async () => {
      const [doc] = await db.select().from(Documents).where(eq(Documents.id, documentId));
      if (!doc) throw new Error('document not found');
      const [set] = await db.select().from(AnalysisPromptSets).where(eq(AnalysisPromptSets.id, promptSetId));
      if (!set) throw new Error('prompt set not found');
      const { survey, selfcheck, defense } = set.content;
      if (!selfcheck || !defense) throw new Error('judge prompt set requires selfcheck and defense');

      const [feedbackSet] = await db
        .select({ id: FeedbackSets.id })
        .from(FeedbackSets)
        .where(and(eq(FeedbackSets.runId, sourceRunId), eq(FeedbackSets.documentId, documentId)));
      if (!feedbackSet) throw new Error('source feedback set not found');

      const rows = await db
        .select()
        .from(Feedbacks)
        .where(and(eq(Feedbacks.setId, feedbackSet.id), eq(Feedbacks.polarity, 'issue')))
        .orderBy(asc(Feedbacks.ord));
      const anchorRows =
        rows.length > 0
          ? await db
              .select()
              .from(FeedbackAnchors)
              .where(
                inArray(
                  FeedbackAnchors.feedbackId,
                  rows.map((r) => r.id),
                ),
              )
              .orderBy(asc(FeedbackAnchors.ord))
          : [];

      const feedbacks: JudgeFeedback[] = rows.map((row) => {
        const anchors = anchorRows.filter((a) => a.feedbackId === row.id).map((a) => ({ quoteStart: a.startText, quoteEnd: a.endText }));
        return {
          id: row.id,
          ord: row.ord,
          category: row.category ?? '',
          body: row.body,
          // 앵커 테이블이 비어 있는 옛 세트는 대표 앵커 컬럼으로 대신한다.
          anchors: anchors.length > 0 ? anchors : [{ quoteStart: row.startText, quoteEnd: row.endText }],
        };
      });

      await db
        .update(PipelineRunDocs)
        .set({ status: 'running', phase: 'survey', error: null, totalChunks: feedbacks.length })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));

      return { content: doc.content, prompts: { survey, selfcheck, defense }, feedbacks };
    });

    const { content, prompts, feedbacks } = resolved;

    try {
      // 원 실행의 survey 캐시가 대부분 소실되어 규약을 다시 만든다. 판정이 읽는 것은 프로필
      // 텍스트뿐이므로 장면 좌표는 해석하지 않는다.
      const survey = await step.do('survey', LLM_STEP, async () => {
        return cachedStep(db, runId, documentId, 'survey', async () => {
          const usage = emptyUsage();
          const raw = await callTool<ResolvedProfile & { nonAnalyticRanges: { startQuote: string; endQuote: string; reason: string }[] }>(
            client,
            prompts.survey,
            SURVEY_TOOL,
            `<원고>\n${content}\n</원고>`,
            usage,
          );

          const findRange = createFindRange(content);
          let cursor = 0;
          const nonAnalyticRanges: WorkProfile['nonAnalyticRanges'] = [];
          for (const item of raw.nonAnalyticRanges) {
            const range = findRange(item.startQuote, item.endQuote, cursor);
            if (!range) continue;
            cursor = range.rangeEnd;
            nonAnalyticRanges.push({ start: range.rangeStart, end: range.rangeEnd, reason: item.reason });
          }

          await addUsage(db, runId, documentId, 'survey', usage);
          return { profile: { ...raw, nonAnalyticRanges } };
        });
      });

      const profile = survey.profile as ResolvedProfile;
      const conventions = renderConventions(renderProfile(profile, null));
      const options = { conventions, manuscript: content, cache: true };

      // ── SELFCHECK ── 근거 판정. 피드백이 말하는 상황이 원고에 실제로 있는가.
      await step.do('phase-selfcheck', () => setPhase(db, runId, documentId, 'selfcheck'));
      const selfchecks: JudgeResult<Selfcheck>[] = [];
      for (let i = 0; i < feedbacks.length; i += JUDGE_FANOUT) {
        const slice = feedbacks.slice(i, i + JUDGE_FANOUT);
        const batch = await step.do(`selfcheck-${i}`, LLM_STEP, async () => {
          const warm = emptyUsage();
          await warmPrefix(client, prompts.selfcheck, SELFCHECK_TOOL, options, warm);
          const results = await Promise.all(
            slice.map((feedback) =>
              cachedCall<JudgeResult<Selfcheck>>(db, runId, documentId, `selfcheck/${feedback.id}`, async (usage) => {
                try {
                  return await callTool<Selfcheck>(
                    client,
                    prompts.selfcheck,
                    SELFCHECK_TOOL,
                    renderSelfCheckInput(feedback, feedback.anchors),
                    usage,
                    options,
                  );
                } catch (err) {
                  return { error: String(err).slice(0, 200) };
                }
              }),
            ),
          );
          await addUsage(db, runId, documentId, 'selfcheck', sumUsage([warm, ...results.map((r) => r.usage)]));
          return results.map((r) => r.value);
        });
        selfchecks.push(...batch);
      }

      // ── DEFENSE ── 항변 판정. 사실이라 해도 작가에게 전달할 가치가 있는가.
      await step.do('phase-defense', () => setPhase(db, runId, documentId, 'defense'));
      const defenses: JudgeResult<Defense>[] = [];
      for (let i = 0; i < feedbacks.length; i += JUDGE_FANOUT) {
        const slice = feedbacks.slice(i, i + JUDGE_FANOUT);
        const batch = await step.do(`defense-${i}`, LLM_STEP, async () => {
          const warm = emptyUsage();
          await warmPrefix(client, prompts.defense, DEFENSE_TOOL, options, warm);
          const results = await Promise.all(
            slice.map((feedback) =>
              cachedCall<JudgeResult<Defense>>(db, runId, documentId, `defense/${feedback.id}`, async (usage) => {
                try {
                  return await callTool<Defense>(
                    client,
                    prompts.defense,
                    DEFENSE_TOOL,
                    renderSelfCheckInput(feedback, feedback.anchors),
                    usage,
                    options,
                  );
                } catch (err) {
                  return { error: String(err).slice(0, 200) };
                }
              }),
            ),
          );
          await addUsage(db, runId, documentId, 'defense', sumUsage([warm, ...results.map((r) => r.usage)]));
          return results.map((r) => r.value);
        });
        defenses.push(...batch);
      }

      await step.do('persist', async () => {
        const results = feedbacks.map((feedback, i) => ({
          feedbackId: feedback.id,
          ord: feedback.ord,
          category: feedback.category,
          selfcheck: selfchecks[i],
          defense: defenses[i],
        }));
        await writeStageCache(db, cacheKey(runId, documentId, 'results'), results);
        await db
          .update(PipelineRunDocs)
          .set({ status: 'done', phase: 'done', doneChunks: feedbacks.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { judged: feedbacks.length };
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
