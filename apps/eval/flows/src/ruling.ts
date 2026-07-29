import { WorkflowEntrypoint } from 'cloudflare:workers';
import { and, asc, eq, inArray } from 'drizzle-orm';
import OpenAI from 'openai';
import { addUsage, cachedCall, cacheKey, emptyUsage, LLM_STEP, sumUsage } from './analysis-llm.ts';
import { renderConventions, renderProfile } from './analysis-render.ts';
import {
  AnalysisPromptSets,
  createDb,
  Documents,
  FeedbackAnchors,
  Feedbacks,
  PipelineRunDocs,
  readStageCache,
  writeStageCache,
} from './db.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { Usage } from './analysis-llm.ts';
import type { ResolvedProfile } from './analysis-render.ts';
import type { FlowEnv, RulingParams } from './index.ts';

const RULING_FANOUT = 16;

// 심판에게는 항변문만 준다. 항변을 만든 쪽의 판정(assessment·verdict)까지 보여주면
// 앵커링이 생겨 벤더 분리가 무의미해진다.
type DefenseItem = { feedbackId: string; ord: number; category: string; defense: { defense?: string; error?: string } };
type RulingItem = { feedbackId: string; ord: number; category: string; body: string; anchors: string[]; defense: string };
type Ruling = { rationale: string; verdict: 'dismiss' | 'uphold' };
type RulingResult = Ruling | { error: string };

const RULING_SCHEMA = {
  name: 'ruling',
  strict: true,
  schema: {
    type: 'object',
    properties: {
      rationale: { type: 'string' },
      verdict: { type: 'string', enum: ['dismiss', 'uphold'] },
    },
    required: ['rationale', 'verdict'],
    additionalProperties: false,
  },
} as const;

const renderRulingInput = (item: RulingItem): string =>
  [
    '<지적>',
    `분류: ${item.category}`,
    item.body,
    '',
    '가리키는 위치:',
    ...item.anchors.map((a, i) => `[${i}] ${a}`),
    '</지적>',
    '',
    '<항변>',
    item.defense,
    '</항변>',
  ].join('\n');

// GPT-5.6은 chat/completions에서 함수 도구 + 추론을 함께 못 쓰지만 structured output과
// 추론 조합은 허용된다(프로브 실측). 도구 대신 response_format으로 판정을 받는다.
const callRuling = async (
  client: OpenAI,
  prompt: { model: string; effort: string | null },
  system: string,
  userContent: string,
  usage: Usage,
): Promise<Ruling> => {
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: system },
      { role: 'user', content: userContent },
    ],
    response_format: { type: 'json_schema', json_schema: RULING_SCHEMA as never },
  };
  if (prompt.effort) (params as Record<string, unknown>).reasoning_effort = prompt.effort;

  const res = await client.chat.completions.create(params, { headers: { 'cf-aig-skip-cache': 'true' } });
  usage.calls += 1;
  usage.promptTokens += res.usage?.prompt_tokens ?? 0;
  usage.cachedTokens += res.usage?.prompt_tokens_details?.cached_tokens ?? 0;
  usage.completionTokens += res.usage?.completion_tokens ?? 0;

  const parsed = JSON.parse(res.choices[0]?.message?.content ?? '') as Ruling;
  if (parsed.verdict !== 'dismiss' && parsed.verdict !== 'uphold') throw new Error(`ruling: verdict 위반 — ${String(parsed.verdict)}`);
  return parsed;
};

// OpenAI 프롬프트 캐싱은 자동이지만 병렬 첫 물결은 서로의 캐시를 못 본다. 접두부(system)를
// 짧은 호출로 먼저 데운다 — 추론을 끄고 출력을 조여 비용을 거의 없앤다.
const warmRuling = async (client: OpenAI, model: string, system: string, usage: Usage): Promise<void> => {
  try {
    const res = await client.chat.completions.create(
      {
        model,
        messages: [
          { role: 'system', content: system },
          { role: 'user', content: '.' },
        ],
        max_completion_tokens: 16,
        reasoning_effort: 'none',
      } as never,
      { headers: { 'cf-aig-skip-cache': 'true' } },
    );
    const u = (res as OpenAI.Chat.Completions.ChatCompletion).usage;
    usage.calls += 1;
    usage.promptTokens += u?.prompt_tokens ?? 0;
    usage.cachedTokens += u?.prompt_tokens_details?.cached_tokens ?? 0;
    usage.completionTokens += u?.completion_tokens ?? 0;
  } catch (err) {
    console.warn(`예열 실패(무시): ${String(err).slice(0, 200)}`);
  }
};

/**
 * 저장된 판정 실행(sourceJudgeRunId)의 항변문에 다른 벤더의 심판만 다시 돌린다.
 *
 * 항변 결과가 피드백 id를 들고 있으므로 지적 본문·앵커는 id로 직접 조회하고, 규약은
 * 원 실행의 survey 캐시를 재사용한다 — 생성은 일절 없다. 결과는 results 키에 피드백
 * id로 남아 사람 라벨과 직접 조인된다.
 */
export class RulingWorkflow extends WorkflowEntrypoint<FlowEnv, RulingParams> {
  async run(event: WorkflowEvent<RulingParams>, step: WorkflowStep) {
    const { runId, promptSetId, sourceJudgeRunId, documentId } = event.payload;
    const db = createDb(this.env.DB);
    const client = new OpenAI({ apiKey: this.env.CLOUDFLARE_API_KEY, baseURL: this.env.CLOUDFLARE_AIGATEWAY_COMPAT_URL });

    const resolved = await step.do('resolve', async () => {
      const [doc] = await db.select().from(Documents).where(eq(Documents.id, documentId));
      if (!doc) throw new Error('document not found');
      const [set] = await db.select().from(AnalysisPromptSets).where(eq(AnalysisPromptSets.id, promptSetId));
      if (!set) throw new Error('prompt set not found');
      const ruling = set.content.ruling;
      if (!ruling) throw new Error('ruling prompt set requires ruling');

      const defenses = await readStageCache<DefenseItem[]>(db, cacheKey(sourceJudgeRunId, documentId, 'results'));
      if (!defenses) throw new Error('source judge results not found');
      const survey = await readStageCache<{ profile: ResolvedProfile }>(db, cacheKey(sourceJudgeRunId, documentId, 'survey'));
      if (!survey) throw new Error('source judge survey not found');

      const ids = defenses.map((d) => d.feedbackId);
      const rows = ids.length > 0 ? await db.select().from(Feedbacks).where(inArray(Feedbacks.id, ids)) : [];
      const anchorRows =
        ids.length > 0
          ? await db.select().from(FeedbackAnchors).where(inArray(FeedbackAnchors.feedbackId, ids)).orderBy(asc(FeedbackAnchors.ord))
          : [];
      const rowOf = new Map(rows.map((r) => [r.id, r]));

      const items: RulingItem[] = [];
      const skipped: { feedbackId: string; error: string }[] = [];
      for (const d of defenses) {
        const row = rowOf.get(d.feedbackId);
        const defense = d.defense.defense;
        if (!row || !defense) {
          skipped.push({ feedbackId: d.feedbackId, error: row ? '항변 없음' : '피드백 없음' });
          continue;
        }
        const anchors = anchorRows.filter((a) => a.feedbackId === d.feedbackId).map((a) => `${a.startText} … ${a.endText}`);
        items.push({
          feedbackId: d.feedbackId,
          ord: d.ord,
          category: row.category ?? '',
          body: row.body,
          anchors: anchors.length > 0 ? anchors : [`${row.startText} … ${row.endText}`],
          defense,
        });
      }

      await db
        .update(PipelineRunDocs)
        .set({ status: 'running', phase: 'ruling', error: null, totalChunks: items.length })
        .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));

      const conventions = renderConventions(renderProfile(survey.profile, null));
      return { content: doc.content, prompt: ruling, items, skipped, conventions };
    });

    const { content, prompt, items, skipped, conventions } = resolved;

    try {
      // 접두부는 문서 단위로 동일해야 OpenAI 자동 캐싱이 문다 — 지시문·규약·원고를
      // 하나의 system으로 고정하고 지적·항변만 user로 보낸다.
      const system = [prompt.system, '', conventions, '', `<원고>\n${content}\n</원고>`].join('\n');

      const rulings: { feedbackId: string; ord: number; category: string; ruling: RulingResult }[] = [];
      for (let i = 0; i < items.length; i += RULING_FANOUT) {
        const slice = items.slice(i, i + RULING_FANOUT);
        const batch = await step.do(`ruling-${i}`, LLM_STEP, async () => {
          const warm = emptyUsage();
          await warmRuling(client, prompt.model, system, warm);
          const results = await Promise.all(
            slice.map((item) =>
              cachedCall<RulingResult>(db, runId, documentId, `ruling/${item.feedbackId}`, async (usage) => {
                try {
                  return await callRuling(client, prompt, system, renderRulingInput(item), usage);
                } catch (err) {
                  return { error: String(err).slice(0, 200) };
                }
              }),
            ),
          );
          await addUsage(db, runId, documentId, 'ruling', sumUsage([warm, ...results.map((r) => r.usage)]));
          return results.map((r) => r.value);
        });
        for (const [k, ruling] of batch.entries()) {
          const item = slice[k];
          rulings.push({ feedbackId: item.feedbackId, ord: item.ord, category: item.category, ruling });
        }
      }

      await step.do('persist', async () => {
        const results = [
          ...rulings,
          ...skipped.map((s) => ({ feedbackId: s.feedbackId, ord: -1, category: '', ruling: { error: s.error } })),
        ];
        await writeStageCache(db, cacheKey(runId, documentId, 'results'), results);
        await db
          .update(PipelineRunDocs)
          .set({ status: 'done', phase: 'done', doneChunks: rulings.length })
          .where(and(eq(PipelineRunDocs.runId, runId), eq(PipelineRunDocs.documentId, documentId)));
      });

      return { ruled: rulings.length };
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
