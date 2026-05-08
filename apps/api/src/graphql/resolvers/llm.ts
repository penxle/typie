import * as Sentry from '@sentry/node';
import { PromptId } from '@typie/lib/const';
import dedent from 'dedent';
import { eq } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import OpenAI from 'openai';
import { dbr, Prompts } from '#/db/index.ts';
import { env } from '#/env.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { builder } from '../builder.ts';

const openai = new OpenAI({
  apiKey: env.CLOUDFLARE_API_KEY,
  baseURL: env.CLOUDFLARE_AIGATEWAY_URL,
});

type Feedback = {
  start: string;
  end: string;
  feedback: string;
};

const provideFeedbackTool: OpenAI.Chat.Completions.ChatCompletionTool = {
  type: 'function',
  function: {
    name: 'provide_feedback',
    description: '현재 분석 구간에서 발견한 피드백 1건을 보고합니다. 피드백할 게 없으면 호출하지 마세요. 여러 건이면 여러 번 호출하세요.',
    parameters: {
      type: 'object',
      properties: {
        start: { type: 'string', description: '구간 시작 문장 (현재 분석할 구간 내 원문 그대로)' },
        end: { type: 'string', description: '구간 끝 문장 (현재 분석할 구간 내 원문 그대로)' },
        feedback: { type: 'string', description: '피드백 본문' },
      },
      required: ['start', 'end', 'feedback'],
    },
  },
};

const loadPrompt = async (id: (typeof PromptId)[keyof typeof PromptId]) => {
  const [prompt] = await dbr
    .select({ model: Prompts.model, effort: Prompts.effort, systemPrompt: Prompts.systemPrompt })
    .from(Prompts)
    .where(eq(Prompts.id, id));
  if (!prompt) {
    throw new Error(`Prompt not found: ${id}`);
  }
  return prompt;
};

const CHUNK_SIZE = 1000;

const createChunks = (text: string) => {
  const chunks: { text: string; start: number; end: number }[] = [];

  let pos = 0;
  while (pos < text.length) {
    let end = pos + CHUNK_SIZE;

    if (end < text.length) {
      const searchStart = Math.max(pos, end - 200);
      let breakPoint = -1;

      for (let i = end; i >= searchStart; i--) {
        if (text[i] === '\n') {
          breakPoint = i + 1;
          break;
        }
      }

      if (breakPoint === -1) {
        const sentencePattern = /[.!?。！？]\s*/g;
        sentencePattern.lastIndex = searchStart;
        let lastMatch = -1;
        let match;

        while ((match = sentencePattern.exec(text)) && match.index <= end) {
          lastMatch = match.index + match[0].length;
        }

        if (lastMatch > pos) {
          breakPoint = lastMatch;
        }
      }

      if (breakPoint > pos) {
        end = breakPoint;
      }
    } else {
      end = text.length;
    }

    chunks.push({
      text: text.slice(pos, end),
      start: pos,
      end,
    });

    pos = end;
  }

  return chunks;
};

const summarizeChunk = async (chunkText: string, signal?: AbortSignal): Promise<string> => {
  const prompt = await loadPrompt(PromptId.SUMMARIZE);
  const response = await openai.chat.completions.create(
    {
      model: prompt.model,
      ...(prompt.effort && { reasoning_effort: prompt.effort as never }),
      messages: [
        { role: 'system', content: prompt.systemPrompt },
        { role: 'user', content: `요약할 텍스트:\n\n${chunkText}` },
      ],
    },
    { signal },
  );

  return response.choices[0]?.message?.content ?? '';
};

type ChunkContext = {
  precedingSummary: string;
  followingSummary: string;
  currentText: string;
};

const extractJsonObjects = function* (buffer: string): Generator<string> {
  let depth = 0;
  let start = -1;
  let inString = false;
  let escape = false;

  for (let i = 0; i < buffer.length; i++) {
    const ch = buffer[i];
    if (inString) {
      if (escape) {
        escape = false;
      } else if (ch === '\\') {
        escape = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
    } else if (ch === '{') {
      if (depth === 0) start = i;
      depth++;
    } else if (ch === '}') {
      depth--;
      if (depth === 0 && start !== -1) {
        yield buffer.slice(start, i + 1);
        start = -1;
      }
    }
  }
};

const analyzeChunkWithContext = async (
  context: ChunkContext,
  onFeedback: (feedback: Feedback) => void,
  signal?: AbortSignal,
): Promise<void> => {
  const userContent = dedent`
    <이전 내용>
    ${context.precedingSummary || '(글의 시작 부분입니다)'}
    </이전 내용>

    <현재 분석할 구간>
    ${context.currentText}
    </현재 분석할 구간>

    <이후 내용>
    ${context.followingSummary || '(글의 마지막 부분입니다)'}
    </이후 내용>
  `;

  const prompt = await loadPrompt(PromptId.ANALYZE);
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: prompt.systemPrompt },
      { role: 'user', content: userContent },
    ],
    tools: [provideFeedbackTool],
    stream: true,
  };
  if (prompt.effort) {
    params.reasoning_effort = prompt.effort as never;
  }
  const stream = await openai.chat.completions.create(params, { signal });

  const accumulators = new Map<number, { name: string; arguments: string }>();

  for await (const chunk of stream) {
    const choice = chunk.choices[0];
    if (!choice) continue;

    for (const delta of choice.delta?.tool_calls ?? []) {
      const acc = accumulators.get(delta.index) ?? { name: '', arguments: '' };
      if (delta.function?.name) acc.name = delta.function.name;
      if (delta.function?.arguments) acc.arguments += delta.function.arguments;
      accumulators.set(delta.index, acc);
    }

    if (choice.finish_reason === 'tool_calls' || choice.finish_reason === 'stop') {
      for (const acc of accumulators.values()) {
        if (acc.name !== 'provide_feedback') continue;
        for (const objStr of extractJsonObjects(acc.arguments)) {
          try {
            const input = JSON.parse(objStr) as Feedback;
            if (input.start && input.end && input.feedback) {
              onFeedback(input);
            }
          } catch (err: unknown) {
            Sentry.captureException(err, { extra: { objStr } });
          }
        }
      }
      accumulators.clear();
    }
  }
};

const LiteraryAnalysisProgress = builder.simpleObject('LiteraryAnalysisProgress', {
  fields: (t) => ({
    current: t.int(),
    total: t.int(),
    phase: t.string(),
  }),
});

const DocumentLiteraryFeedbackResult = builder.simpleObject('DocumentLiteraryFeedbackResult', {
  fields: (t) => ({
    nodeId: t.string(),
    startOffset: t.int(),
    endOffset: t.int(),
    startText: t.string(),
    endText: t.string(),
    feedback: t.string(),
  }),
});

type DocumentAnalysisPayload =
  | {
      type: 'feedback';
      data: { nodeId: string; startOffset: number; endOffset: number; startText: string; endText: string; feedback: string };
    }
  | { type: 'progress'; data: { current: number; total: number; phase: 'summarizing' | 'analyzing' } }
  | { type: 'complete' }
  | { type: 'error' };

const DocumentTextMappingInput = builder.inputType('DocumentTextMappingInput', {
  fields: (t) => ({
    nodeId: t.string(),
    textStart: t.int(),
    textEnd: t.int(),
    blockOffset: t.int(),
  }),
});

const createMapRangeForDocument = (
  text: string,
  mappings: { nodeId: string; textStart: number; textEnd: number; blockOffset: number }[],
) => {
  const findMapping = (position: number) => {
    let left = 0;
    let right = mappings.length - 1;

    while (left <= right) {
      const mid = (left + right) >> 1;
      const m = mappings[mid];

      if (position >= m.textStart && position < m.textEnd) {
        return m;
      }

      if (position < m.textStart) {
        right = mid - 1;
      } else {
        left = mid + 1;
      }
    }

    return;
  };

  return (startText: string, endText: string, searchStart = 0) => {
    const rangeStart = text.indexOf(startText, searchStart);
    if (rangeStart === -1) {
      return null;
    }

    const endSearchStart = startText === endText ? rangeStart : rangeStart + startText.length;
    const endIndex = text.indexOf(endText, endSearchStart);
    if (endIndex === -1) {
      return null;
    }
    const rangeEnd = endIndex + endText.length;

    const startMapping = findMapping(rangeStart);
    const endMapping = findMapping(rangeEnd - 1);

    if (!startMapping || !endMapping || startMapping.nodeId !== endMapping.nodeId) {
      return null;
    }

    return {
      nodeId: startMapping.nodeId,
      startOffset: startMapping.blockOffset + (rangeStart - startMapping.textStart),
      endOffset: startMapping.blockOffset + (rangeEnd - startMapping.textStart),
    };
  };
};

builder.subscriptionFields((t) => ({
  literaryAnalysisDocumentStream: t.withAuth({ session: true }).field({
    type: builder.simpleObject('DocumentLiteraryAnalysisPayload', {
      fields: (t) => ({
        type: t.string(),
        feedback: t.field({ type: DocumentLiteraryFeedbackResult, nullable: true }),
        progress: t.field({ type: LiteraryAnalysisProgress, nullable: true }),
      }),
    }),
    args: {
      text: t.arg.string(),
      mappings: t.arg({ type: [DocumentTextMappingInput] }),
    },
    subscribe: async (_, args, ctx) => {
      await assertActiveSubscription({ userId: ctx.session.userId });

      const text = args.text;
      const mappings = args.mappings;

      return new Repeater<DocumentAnalysisPayload>(async (push, stop) => {
        const abortController = new AbortController();
        const signal = abortController.signal;

        ctx.c.req.raw.signal.addEventListener('abort', () => {
          abortController.abort();
          stop();
        });

        if (!text.trim()) {
          push({ type: 'complete' });
          stop();
          return;
        }

        const chunks = createChunks(text);
        const mapRange = createMapRangeForDocument(text, mappings);

        try {
          const summaries: string[] = [];
          await Promise.all(
            chunks.map(async (chunk, index) => {
              signal.throwIfAborted();
              const summary = await summarizeChunk(chunk.text, signal);
              summaries[index] = summary;
              push({
                type: 'progress',
                data: { current: summaries.filter(Boolean).length, total: chunks.length, phase: 'summarizing' },
              });
            }),
          );

          let analyzedCount = 0;
          push({
            type: 'progress',
            data: { current: 0, total: chunks.length, phase: 'analyzing' },
          });
          await Promise.all(
            chunks.map(async (chunk, i) => {
              signal.throwIfAborted();
              const precedingSummary = summaries.slice(0, i).join('\n\n');
              const followingSummary = summaries.slice(i + 1).join('\n\n');

              await analyzeChunkWithContext(
                {
                  precedingSummary,
                  followingSummary,
                  currentText: chunk.text,
                },
                (feedback) => {
                  const range = mapRange(feedback.start, feedback.end, chunk.start);

                  if (range) {
                    push({
                      type: 'feedback',
                      data: {
                        nodeId: range.nodeId,
                        startOffset: range.startOffset,
                        endOffset: range.endOffset,
                        startText: feedback.start,
                        endText: feedback.end,
                        feedback: feedback.feedback,
                      },
                    });
                  }
                },
                signal,
              );

              analyzedCount++;
              push({
                type: 'progress',
                data: { current: analyzedCount, total: chunks.length, phase: 'analyzing' },
              });
            }),
          );

          push({ type: 'complete' });
        } catch (err) {
          if (!signal.aborted) {
            Sentry.captureException(err);
            console.error(err);
            push({ type: 'error' });
          }
        }

        stop();
      });
    },
    resolve: (payload: DocumentAnalysisPayload) => {
      return {
        type: payload.type,
        feedback: payload.type === 'feedback' ? payload.data : null,
        progress: payload.type === 'progress' ? payload.data : null,
      };
    },
  }),
}));
