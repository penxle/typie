import OpenAI from 'openai';
import { dedupCharacterCandidates, extractJsonObjects, renderMetaBlock, renderSummaryForMeta } from './text.ts';
import type { StagePrompt } from '../../src/lib/domain/admin-types.ts';
import type { Feedback, MetaStructured, SummaryStructured, ToolDescriptions } from './text.ts';

export type Usage = { promptTokens: number; completionTokens: number; cachedTokens: number };

export type ResolvedPrompt = {
  model: string;
  effort: string | null;
  temperature: number | null;
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

// temperature는 값이 있을 때만 해시에 들어간다 — 온도를 지정하지 않은 기존 단계의 해시가 그대로여야
// stage_cache가 계속 히트한다.
export const hashStagePrompt = async (stage: StagePrompt): Promise<string> => {
  const parts: unknown[] = [stage.model, stage.effort, stage.system, stage.tools];
  if (stage.temperature !== undefined && stage.temperature !== null) {
    parts.push(stage.temperature);
  }
  const encoded = new TextEncoder().encode(JSON.stringify(parts));
  const digest = await crypto.subtle.digest('SHA-256', encoded);
  return toHex(digest).slice(0, 16);
};

export const resolveStagePrompt = async (stage: StagePrompt): Promise<ResolvedPrompt> => ({
  model: stage.model,
  effort: stage.effort,
  temperature: stage.temperature ?? null,
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
  if (prompt.temperature !== null) {
    params.temperature = prompt.temperature;
  }
  const response = await openai.chat.completions.create(params);
  if (response.usage) {
    usage.promptTokens += response.usage.prompt_tokens ?? 0;
    usage.completionTokens += response.usage.completion_tokens ?? 0;
    usage.cachedTokens += response.usage.prompt_tokens_details?.cached_tokens ?? 0;
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
  if (prompt.temperature !== null) {
    params.temperature = prompt.temperature;
  }
  const stream = await openai.chat.completions.create(params);

  const accumulators = new Map<number, { name: string; arguments: string }>();

  for await (const chunk of stream) {
    if (chunk.usage) {
      usage.promptTokens += chunk.usage.prompt_tokens ?? 0;
      usage.completionTokens += chunk.usage.completion_tokens ?? 0;
      usage.cachedTokens += chunk.usage.prompt_tokens_details?.cached_tokens ?? 0;
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
