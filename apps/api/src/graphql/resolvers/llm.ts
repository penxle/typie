import * as Sentry from '@sentry/node';
import { PromptId } from '@typie/lib/const';
import { eq } from 'drizzle-orm';
import escape from 'escape-string-regexp';
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
  category?: string;
};

type SummaryStructured = {
  narrative: string;
  characters: string[];
  pov: string;
  tense: string;
  location: string;
  tone: string;
};

type MetaStructured = {
  narrator: { pov: string; reliability: string };
  setting: string;
  themes: string[];
  characters: { name: string; aliases: string[]; role: string; arc: string }[];
  structure: { label: string; summary: string; tone: string }[];
  style: string;
};

type AnalysisPhase = 'summarizing' | 'meta' | 'analyzing';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ToolDescriptions = Record<string, any>;

const buildFeedbackTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_feedback',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        start: { type: 'string', description: d.start },
        end: { type: 'string', description: d.end },
        feedback: { type: 'string', description: d.feedback },
        category: { type: 'string', description: d.category },
      },
      required: ['start', 'end', 'feedback'],
    },
  },
});

const buildSummaryTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_summary',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        narrative: { type: 'string', description: d.narrative },
        characters: { type: 'array', items: { type: 'string' }, description: d.characters },
        pov: { type: 'string', description: d.pov },
        tense: { type: 'string', description: d.tense },
        location: { type: 'string', description: d.location },
        tone: { type: 'string', description: d.tone },
      },
    },
  },
});

const buildMetaTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_meta',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        narrator: {
          type: 'object',
          description: d.narrator.self,
          properties: {
            pov: { type: 'string', description: d.narrator.pov },
            reliability: { type: 'string', description: d.narrator.reliability },
          },
        },
        setting: { type: 'string', description: d.setting },
        themes: { type: 'array', items: { type: 'string' }, description: d.themes },
        characters: {
          type: 'array',
          description: d.characters.self,
          items: {
            type: 'object',
            properties: {
              name: { type: 'string', description: d.characters.name },
              aliases: { type: 'array', items: { type: 'string' }, description: d.characters.aliases },
              role: { type: 'string', description: d.characters.role },
              arc: { type: 'string', description: d.characters.arc },
            },
          },
        },
        structure: {
          type: 'array',
          description: d.structure.self,
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: d.structure.label },
              summary: { type: 'string', description: d.structure.summary },
              tone: { type: 'string', description: d.structure.tone },
            },
          },
        },
        style: { type: 'string', description: d.style },
      },
    },
  },
});

const loadPrompt = async (id: (typeof PromptId)[keyof typeof PromptId]) => {
  const [prompt] = await dbr
    .select({
      model: Prompts.model,
      effort: Prompts.effort,
      systemPrompt: Prompts.systemPrompt,
      toolDescriptions: Prompts.toolDescriptions,
    })
    .from(Prompts)
    .where(eq(Prompts.id, id));
  if (!prompt) {
    throw new Error(`Prompt not found: ${id}`);
  }
  if (!prompt.toolDescriptions) {
    throw new Error(`Prompt ${id} missing tool_descriptions`);
  }
  return prompt;
};

type Prompt = Awaited<ReturnType<typeof loadPrompt>>;

const runTool = async <T>(
  prompt: Prompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  userContent: string,
  signal?: AbortSignal,
): Promise<T> => {
  const toolName = tool.function.name;
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: prompt.systemPrompt },
      { role: 'user', content: userContent },
    ],
    tools: [tool],
    tool_choice: { type: 'function', function: { name: toolName } },
  };
  if (prompt.effort) {
    params.reasoning_effort = prompt.effort as never;
  }
  const response = await openai.chat.completions.create(params, { signal });

  const toolCall = response.choices[0]?.message?.tool_calls?.[0];
  if (!toolCall || toolCall.type !== 'function' || toolCall.function.name !== toolName) {
    throw new Error(`${toolName} tool call missing`);
  }
  return JSON.parse(toolCall.function.arguments) as T;
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

type ChunkContext = {
  meta: MetaStructured;
  precedingNarrative: string;
  followingNarrative: string;
  currentText: string;
};

const extractJsonObjects = function* (buffer: string): Generator<string> {
  let depth = 0;
  let start = -1;
  let inString = false;
  let escapeNext = false;

  for (let i = 0; i < buffer.length; i++) {
    const ch = buffer[i];
    if (inString) {
      if (escapeNext) {
        escapeNext = false;
      } else if (ch === '\\') {
        escapeNext = true;
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

const dedupCharacterCandidates = (summaries: SummaryStructured[]): string[] => {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const s of summaries) {
    for (const name of s.characters ?? []) {
      const normalized = name.trim().replaceAll(/^["']|["']$/g, '');
      if (!normalized) continue;
      const key = normalized.toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        result.push(normalized);
      }
    }
  }
  return result;
};

const renderSummaryForMeta = (summary: SummaryStructured): string => {
  const characters = summary.characters ?? [];

  const meta1: string[] = [];
  if (characters.length > 0) meta1.push(`인물: ${characters.join(', ')}`);
  if (summary.pov) meta1.push(`시점: ${summary.pov}`);
  if (summary.tense) meta1.push(`시제: ${summary.tense}`);

  const meta2: string[] = [];
  if (summary.location) meta2.push(`장소: ${summary.location}`);
  if (summary.tone) meta2.push(`분위기: ${summary.tone}`);

  const lines: string[] = [];
  if (summary.narrative) lines.push(summary.narrative);
  if (meta1.length > 0) lines.push(meta1.map((m) => `[${m}]`).join(' '));
  if (meta2.length > 0) lines.push(meta2.map((m) => `[${m}]`).join(' '));

  return lines.join('\n');
};

const renderMetaBlock = (meta: MetaStructured): string => {
  const characterLines = (meta.characters ?? []).map((c) => {
    const aliases = c.aliases ?? [];
    const aliasPart = aliases.length > 0 ? ` (${aliases.join('/')})` : '';
    return `- ${c.name ?? ''}${aliasPart}: ${c.role ?? ''}. ${c.arc ?? ''}`;
  });
  const structureLines = (meta.structure ?? []).map((s) => `- ${s.label ?? ''}: ${s.summary ?? ''} [${s.tone ?? ''}]`);

  return [
    '<작품 전체>',
    `서술 시점: ${meta.narrator?.pov ?? ''}`,
    `화자 신뢰성: ${meta.narrator?.reliability ?? ''}`,
    `배경: ${meta.setting ?? ''}`,
    `주제: ${(meta.themes ?? []).join(', ')}`,
    `문체: ${meta.style ?? ''}`,
    '',
    '등장인물:',
    ...characterLines,
    '',
    '구조:',
    ...structureLines,
    '</작품 전체>',
  ].join('\n');
};

const analyzeGlobal = async (
  prompt: Prompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  summaries: SummaryStructured[],
  signal?: AbortSignal,
): Promise<MetaStructured> => {
  const summaryBlocks = summaries.map((s, i) => `[${i + 1}]\n${renderSummaryForMeta(s)}`).join('\n\n');
  const userContent = [
    '<인물 후보>',
    dedupCharacterCandidates(summaries).join(', '),
    '</인물 후보>',
    '',
    '<청크별 요약>',
    summaryBlocks,
    '</청크별 요약>',
  ].join('\n');

  return runTool<MetaStructured>(prompt, tool, userContent, signal);
};

const analyzeChunkWithContext = async (
  prompt: Prompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  context: ChunkContext,
  onFeedback: (feedback: Feedback) => void,
  signal?: AbortSignal,
): Promise<void> => {
  const userContent = [
    renderMetaBlock(context.meta),
    '',
    '<이전 내용>',
    context.precedingNarrative || '(글의 시작 부분입니다)',
    '</이전 내용>',
    '',
    '<현재 분석할 구간>',
    context.currentText,
    '</현재 분석할 구간>',
    '',
    '<이후 내용>',
    context.followingNarrative || '(글의 마지막 부분입니다)',
    '</이후 내용>',
  ].join('\n');

  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsStreaming = {
    model: prompt.model,
    messages: [
      { role: 'system', content: prompt.systemPrompt },
      { role: 'user', content: userContent },
    ],
    tools: [tool],
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
    category: t.string({ nullable: true }),
  }),
});

type DocumentAnalysisPayload =
  | {
      type: 'feedback';
      data: {
        nodeId: string;
        startOffset: number;
        endOffset: number;
        startText: string;
        endText: string;
        feedback: string;
        category: string | null;
      };
    }
  | { type: 'progress'; data: { current: number; total: number; phase: AnalysisPhase } }
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

type Match = { index: number; length: number };

const fuzzyFindMatch = (haystack: string, needle: string, fromIndex: number): Match | null => {
  const trimmed = needle.trim();
  if (!trimmed) return null;
  const pattern = escape(trimmed).replaceAll(/\s+/g, String.raw`\s+`);
  const subStart = Math.max(0, fromIndex);
  const match = new RegExp(pattern).exec(haystack.slice(subStart));
  if (!match) return null;
  return { index: subStart + match.index, length: match[0].length };
};

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

  const exactFind = (needle: string, from: number): Match | null => {
    const idx = text.indexOf(needle, from);
    return idx === -1 ? null : { index: idx, length: needle.length };
  };

  return (startText: string, endText: string, searchStart = 0) => {
    const findRange = (find: (needle: string, from: number) => Match | null) => {
      const start = find(startText, searchStart);
      if (!start) return null;
      const endFrom = startText === endText ? start.index : start.index + start.length;
      const end = find(endText, endFrom);
      if (!end) return null;
      return { rangeStart: start.index, rangeEnd: end.index + end.length };
    };

    const range = findRange(exactFind) ?? findRange((n, from) => fuzzyFindMatch(text, n, from));
    if (!range) {
      Sentry.captureMessage('literary feedback range match failed', {
        level: 'warning',
        extra: { startText, endText, searchStart },
      });
      return null;
    }

    const { rangeStart, rangeEnd } = range;
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
          const [summarizePrompt, metaPrompt, analyzePrompt] = await Promise.all([
            loadPrompt(PromptId.SUMMARIZE),
            loadPrompt(PromptId.META),
            loadPrompt(PromptId.ANALYZE),
          ]);
          const summaryTool = buildSummaryTool(summarizePrompt.toolDescriptions as ToolDescriptions);
          const metaTool = buildMetaTool(metaPrompt.toolDescriptions as ToolDescriptions);
          const feedbackTool = buildFeedbackTool(analyzePrompt.toolDescriptions as ToolDescriptions);

          const summaries: SummaryStructured[] = [];
          let summarizedCount = 0;
          await Promise.all(
            chunks.map(async (chunk, index) => {
              signal.throwIfAborted();
              summaries[index] = await runTool<SummaryStructured>(summarizePrompt, summaryTool, chunk.text, signal);
              summarizedCount++;
              push({
                type: 'progress',
                data: { current: summarizedCount, total: chunks.length, phase: 'summarizing' },
              });
            }),
          );

          push({ type: 'progress', data: { current: 0, total: 1, phase: 'meta' } });
          signal.throwIfAborted();
          const meta = await analyzeGlobal(metaPrompt, metaTool, summaries, signal);
          push({ type: 'progress', data: { current: 1, total: 1, phase: 'meta' } });

          let analyzedCount = 0;
          push({
            type: 'progress',
            data: { current: 0, total: chunks.length, phase: 'analyzing' },
          });
          await Promise.all(
            chunks.map(async (chunk, i) => {
              signal.throwIfAborted();
              const precedingNarrative = i > 0 ? (summaries[i - 1].narrative ?? '') : '';
              const followingNarrative = i < chunks.length - 1 ? (summaries[i + 1].narrative ?? '') : '';

              await analyzeChunkWithContext(
                analyzePrompt,
                feedbackTool,
                {
                  meta,
                  precedingNarrative,
                  followingNarrative,
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
                        category: feedback.category ?? null,
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
