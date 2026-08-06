// OpenAI 호환 경로. 게이트웨이 compat 엔드포인트는 모델 문자열(`{provider}/{model}`)을 그대로
// 받으므로 접두사를 벗기지 않는다. Anthropic 경로와 다른 점 셋 — 대화가 chat.completions
// 형태라 여기서 변환하고, 캐시 표시가 없으며(프로바이더의 암시적 캐싱에 맡긴다), 쓰기 할증이
// 없다(usage에 cacheWrite를 더하지 않는다 — 원장·게이트웨이 대조로 검증된 매핑).
import { schemaViolations } from '../tool-schema.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type OpenAI from 'openai';
import type { PhasePrompt, Usage } from '../contracts.ts';
import type { ToolUse, TurnOutput } from './agent-loop.ts';

// 두 경로의 클라이언트를 함께 든다. 어느 쪽으로 나갈지는 호출이 아니라 모델이 정한다.
export type LlmClients = { anthropic: Anthropic; compat: OpenAI };

export const isAnthropicModel = (model: string): boolean => model.startsWith('anthropic/');

const SKIP_CACHE = { headers: { 'cf-aig-skip-cache': 'true' } };

// strict는 옮기지 않는다. OpenAI strict는 스키마 요건이 달라 400을 낼 수 있고, 위반은
// schemaViolations 재시도가 잡는다.
export const toCompatTools = (tools: Anthropic.Messages.Tool[]): OpenAI.Chat.Completions.ChatCompletionFunctionTool[] =>
  tools.map((tool) => ({
    type: 'function',
    function: { name: tool.name, description: tool.description, parameters: tool.input_schema as unknown as Record<string, unknown> },
  }));

// 인자 파싱 실패는 빈 입력으로 대체한다 — 제출이면 스키마 반려 루프가, 로컬 도구면 오류
// 결과가 모델에게 되돌아가 스스로 고치게 된다.
const parseArguments = (name: string, raw: string): unknown => {
  try {
    return JSON.parse(raw || '{}');
  } catch {
    console.warn(`${name}: 도구 인자 JSON 파싱 실패 — 빈 입력으로 대체`);
    return {};
  }
};

// 루프의 대화 상태는 Anthropic 블록 형태 하나로 유지하고, 나갈 때만 변환한다. cache_control
// 표시는 여기서 자연히 떨어져 나간다. tool_result만 담긴 user 메시지는 role:tool 나열로,
// 문구가 섞이면 tool 다음에 user로 낸다 — tool 메시지는 tool_calls 직후여야 한다.
export const toCompatMessages = (
  systems: (string | null | undefined)[],
  messages: Anthropic.MessageParam[],
): OpenAI.Chat.Completions.ChatCompletionMessageParam[] => {
  const out: OpenAI.Chat.Completions.ChatCompletionMessageParam[] = [
    { role: 'system', content: systems.filter((s): s is string => typeof s === 'string' && s.length > 0).join('\n\n') },
  ];
  for (const message of messages) {
    if (typeof message.content === 'string') {
      out.push({ role: message.role, content: message.content });
      continue;
    }
    if (message.role === 'user') {
      const texts: string[] = [];
      for (const block of message.content) {
        if (block.type === 'tool_result') {
          out.push({
            role: 'tool',
            tool_call_id: block.tool_use_id,
            content: typeof block.content === 'string' ? block.content : JSON.stringify(block.content ?? ''),
          });
        } else if (block.type === 'text') {
          texts.push(block.text);
        }
      }
      if (texts.length > 0) out.push({ role: 'user', content: texts.join('\n') });
      continue;
    }
    const texts: string[] = [];
    let messageExtra: unknown;
    const toolCalls: OpenAI.Chat.Completions.ChatCompletionMessageFunctionToolCall[] = [];
    for (const block of message.content) {
      // Gemini 3는 함수 호출에 thought_signature(extra_content)를 붙여 주고, 이력을 되돌릴 때
      // 그대로 포함하지 않으면 400을 낸다. 응답에서 블록에 실어 둔 것을 여기서 복원한다.
      const extra = (block as { extra_content?: unknown }).extra_content;
      if (block.type === 'text') {
        texts.push(block.text);
        if (extra !== undefined) messageExtra = extra;
      } else if (block.type === 'tool_use') {
        toolCalls.push({
          id: block.id,
          type: 'function',
          function: { name: block.name, arguments: JSON.stringify(block.input ?? {}) },
          ...(extra !== undefined && { extra_content: extra }),
        } as OpenAI.Chat.Completions.ChatCompletionMessageFunctionToolCall);
      }
    }
    // 완전히 빈 턴(텍스트도 도구 호출도 없음)은 이력에서 뺀다 — Gemini가 빈 parts를 400으로
    // 거부하고, 담긴 정보도 없다. 모델은 이따금 빈 응답을 내며 루프가 재촉 문구로 잇는다.
    if (texts.length === 0 && toolCalls.length === 0) continue;
    out.push({
      role: 'assistant',
      content: texts.length > 0 ? texts.join('\n') : null,
      ...(toolCalls.length > 0 && { tool_calls: toolCalls }),
      ...(messageExtra !== undefined && { extra_content: messageExtra }),
    } as OpenAI.Chat.Completions.ChatCompletionMessageParam);
  }
  return out;
};

// 응답을 루프가 아는 블록 형태로 되돌린다 — 다음 턴 변환과 오염 스크럽이 같은 모양을 본다.
export const turnFromCompat = (message: OpenAI.Chat.Completions.ChatCompletionMessage | undefined): TurnOutput => {
  const content: unknown[] = [];
  const messageExtra = (message as { extra_content?: unknown } | undefined)?.extra_content;
  if (message?.content) {
    content.push({ type: 'text', text: message.content, ...(messageExtra !== undefined && { extra_content: messageExtra }) });
  }
  const toolUses: ToolUse[] = [];
  for (const call of message?.tool_calls ?? []) {
    if (call.type !== 'function') continue;
    // 서명(extra_content)은 대화 상태에 블록과 함께 실어 D1 캐시·리플레이를 그대로 통과시킨다.
    const extra = (call as { extra_content?: unknown }).extra_content;
    const use = { id: call.id, name: call.function.name, input: parseArguments(call.function.name, call.function.arguments) };
    toolUses.push(use);
    content.push({ type: 'tool_use', id: use.id, name: use.name, input: use.input, ...(extra !== undefined && { extra_content: extra }) });
  }
  return { content, toolUses };
};

export const addCompatUsage = (usage: Usage, u: OpenAI.Completions.CompletionUsage | undefined | null): void => {
  usage.calls += 1;
  usage.promptTokens += u?.prompt_tokens ?? 0;
  usage.cachedTokens += u?.prompt_tokens_details?.cached_tokens ?? 0;
  usage.completionTokens += u?.completion_tokens ?? 0;
};

const withEffort = <T extends object>(params: T, prompt: PhasePrompt): T => {
  if (prompt.effort) (params as Record<string, unknown>).reasoning_effort = prompt.effort;
  return params;
};

export const runTurnCompat = async (
  client: OpenAI,
  prompt: PhasePrompt,
  tools: Anthropic.Messages.Tool[],
  system: string,
  messages: Anthropic.MessageParam[],
  usage: Usage,
): Promise<TurnOutput> => {
  const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = withEffort(
    {
      model: prompt.model,
      messages: toCompatMessages([prompt.system, system], messages),
      tools: toCompatTools(tools),
      tool_choice: 'auto',
    },
    prompt,
  );
  // 빈 응답(텍스트도 도구 호출도 없음)은 한 번 바로 다시 받는다 — 그대로 캐시되면 리플레이마다
  // 턴만 낭비된다. 두 번째도 비면 루프의 재촉에 맡긴다.
  let turn: TurnOutput = { content: [], toolUses: [] };
  for (let attempt = 0; attempt < 2; attempt++) {
    const res = await client.chat.completions.create(params, SKIP_CACHE);
    addCompatUsage(usage, res.usage);
    turn = turnFromCompat(res.choices[0]?.message);
    if (turn.content.length > 0 || turn.toolUses.length > 0) return turn;
    console.warn(`${prompt.model}: 빈 응답 (시도 ${attempt + 1})`);
  }
  return turn;
};

// 단발 강제 도구 호출 — Anthropic 경로의 callTool과 같은 계약(스키마 위반 1회 재시도).
export const callToolCompat = async <T>(
  client: OpenAI,
  prompt: PhasePrompt,
  tool: Anthropic.Messages.Tool,
  systems: (string | null | undefined)[],
  userContent: string,
  usage: Usage,
): Promise<T> => {
  const messages: OpenAI.Chat.Completions.ChatCompletionMessageParam[] = [
    ...toCompatMessages(systems, []),
    { role: 'user', content: userContent },
  ];

  let violations: string[] = [];
  for (let attempt = 0; attempt < 2; attempt++) {
    const params: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = withEffort(
      {
        model: prompt.model,
        messages,
        tools: toCompatTools([tool]),
        tool_choice: { type: 'function', function: { name: tool.name } },
      },
      prompt,
    );
    const res = await client.chat.completions.create(params, SKIP_CACHE);
    addCompatUsage(usage, res.usage);

    const message = res.choices[0]?.message;
    const call = (message?.tool_calls ?? []).find((c) => c.type === 'function' && c.function.name === tool.name);
    if (!call || call.type !== 'function') throw new Error(`${tool.name}: 도구 호출 없음`);

    const parsed = parseArguments(call.function.name, call.function.arguments) as T;
    violations = schemaViolations(tool.input_schema, parsed);
    if (violations.length === 0) return parsed;

    console.warn(`${tool.name}: 스키마 위반 ${violations.length}건 (시도 ${attempt + 1}) — ${violations.slice(0, 5).join(' / ')}`);
    messages.push(
      { role: 'assistant', content: message?.content ?? null, tool_calls: message?.tool_calls },
      { role: 'tool', tool_call_id: call.id, content: `스키마를 어겼습니다.\n${violations.join('\n')}\n전체를 다시 채워 보내세요.` },
    );
  }

  throw new Error(`${tool.name}: 스키마 위반 ${violations.join(' / ')}`);
};
