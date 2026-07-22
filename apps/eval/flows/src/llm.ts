import OpenAI from 'openai';
import { dedupCharacterCandidates, extractJsonObjects, renderMetaBlock, renderSummaryForMeta } from './text.ts';
import type { StagePrompt } from '../../src/lib/domain/admin-types.ts';
import type { Feedback, MetaStructured, SummaryStructured, ToolDescriptions } from './text.ts';

export type Usage = { promptTokens: number; completionTokens: number };

export type ResolvedPrompt = {
  model: string;
  effort: string | null;
  systemPrompt: string;
  toolDescriptions: ToolDescriptions;
  hash: string;
};

export type ChunkContext = {
  meta: MetaStructured;
  precedingNarrative: string;
  followingNarrative: string;
  currentText: string;
};

export const createOpenAI = (apiKey: string, baseURL: string): OpenAI => new OpenAI({ apiKey, baseURL });

const toHex = (buffer: ArrayBuffer): string => [...new Uint8Array(buffer)].map((b) => b.toString(16).padStart(2, '0')).join('');

export const hashStagePrompt = async (stage: StagePrompt): Promise<string> => {
  const encoded = new TextEncoder().encode(JSON.stringify([stage.model, stage.effort, stage.system, stage.tools]));
  const digest = await crypto.subtle.digest('SHA-256', encoded);
  return toHex(digest).slice(0, 16);
};

export const resolveStagePrompt = async (stage: StagePrompt): Promise<ResolvedPrompt> => ({
  model: stage.model,
  effort: stage.effort,
  systemPrompt: stage.system,
  toolDescriptions: stage.tools as ToolDescriptions,
  hash: await hashStagePrompt(stage),
});

export const runTool = async <T>(
  prompt: ResolvedPrompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  userContent: string,
  openai: OpenAI,
  usage: Usage,
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
  const response = await openai.chat.completions.create(params);
  if (response.usage) {
    usage.promptTokens += response.usage.prompt_tokens;
    usage.completionTokens += response.usage.completion_tokens;
  }

  const toolCall = response.choices[0]?.message?.tool_calls?.[0];
  if (!toolCall || toolCall.type !== 'function' || toolCall.function.name !== toolName) {
    throw new Error(`${toolName} tool call missing`);
  }
  return JSON.parse(toolCall.function.arguments) as T;
};

export const analyzeGlobal = async (
  prompt: ResolvedPrompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  summaries: SummaryStructured[],
  openai: OpenAI,
  usage: Usage,
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

  return runTool<MetaStructured>(prompt, tool, userContent, openai, usage);
};

export const analyzeChunkWithContext = async (
  prompt: ResolvedPrompt,
  tool: OpenAI.Chat.Completions.ChatCompletionFunctionTool,
  context: ChunkContext,
  onFeedback: (feedback: Feedback) => void,
  openai: OpenAI,
  usage: Usage,
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
    stream_options: { include_usage: true },
  };
  if (prompt.effort) {
    params.reasoning_effort = prompt.effort as never;
  }
  const stream = await openai.chat.completions.create(params);

  const accumulators = new Map<number, { name: string; arguments: string }>();

  for await (const chunk of stream) {
    if (chunk.usage) {
      usage.promptTokens += chunk.usage.prompt_tokens;
      usage.completionTokens += chunk.usage.completion_tokens;
    }

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
            console.warn(`feedback JSON parse failed (${objStr.length} chars): ${String(err)}`);
          }
        }
      }
      accumulators.clear();
    }
  }
};

const SYSTEM_PROMPT = [
  '당신은 텍스트 분류기입니다.',
  '제공된 텍스트가 문학적 창작물(소설, 시, 수필, 희곡, 시나리오 등 서사와 정서 표현이 중심인 글)인지 판별하세요.',
  '메모, 일기, 할 일 목록, 정보 전달 글, 설정 노트, 강의 자료, 리뷰, 기사, 공지문은 문학적 창작물이 아닙니다.',
  '세계관 설정이나 인물 소개만 나열된 글도 문학적 창작물이 아닙니다. 실제 서사가 전개되어야 합니다.',
  'classify 도구를 정확히 한 번 호출하세요. 도구 호출 외의 텍스트 응답은 불필요합니다.',
].join('\n');

const classifyTool: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'classify',
    description: '텍스트의 문학성 판별 결과를 보고합니다.',
    parameters: {
      type: 'object',
      properties: {
        literary: { type: 'boolean', description: '문학적 창작물이면 true' },
        kind: { type: 'string', description: "글의 종류. 예: '소설', '수필', '시', '일기', '메모', '정보글'" },
      },
      required: ['literary', 'kind'],
    },
  },
};

export const classifyLiterary = async (openai: OpenAI, model: string, text: string): Promise<{ literary: boolean; kind: string }> => {
  const response = await openai.chat.completions.create({
    model,
    messages: [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: text.slice(0, 2000) },
    ],
    tools: [classifyTool],
    tool_choice: { type: 'function', function: { name: 'classify' } },
  });
  const call = response.choices[0]?.message?.tool_calls?.[0];
  if (!call || call.type !== 'function' || call.function.name !== 'classify') {
    throw new Error('classify tool call missing');
  }
  return JSON.parse(call.function.arguments) as { literary: boolean; kind: string };
};
