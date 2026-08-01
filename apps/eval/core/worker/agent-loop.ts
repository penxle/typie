// 에이전틱 턴 실행기. 모델 턴 하나를 돌리고, 비결정적 도구(search)를 실행해 tool_result로
// 되돌린다. 파일 도구(read/grep/write/edit)는 워크스페이스의 순수 적용이고 제출은 스테이지
// 루프의 몫이다 — 여기는 캐시가 필요한 검색만 안다.
import { isAnthropicModel, runTurnCompat } from './compat.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { PhasePrompt, ToolRecord, Usage } from '../contracts.ts';
import type { LlmClients } from './compat.ts';

const MAX_OUTPUT_TOKENS = 64_000;
const SKIP_CACHE = { headers: { 'cf-aig-skip-cache': 'true' } };

const modelId = (model: string): string => model.replace(/^anthropic\//, '');

export type ToolUse = { id: string; name: string; input: unknown };
export type TurnOutput = { content: unknown[]; toolUses: ToolUse[] };

// 대화 접두부가 턴마다 캐시되도록 마지막 메시지의 마지막 블록에 표시를 옮겨 단다.
// 이것이 없으면 대화 비용이 턴 수에 제곱으로 는다.
const withLastBlockCached = (messages: Anthropic.MessageParam[]): Anthropic.MessageParam[] =>
  messages.map((m, i) => {
    if (i !== messages.length - 1) return m;
    const blocks: Record<string, unknown>[] =
      typeof m.content === 'string'
        ? [{ type: 'text', text: m.content }]
        : m.content.map((b) => ({ ...(b as unknown as Record<string, unknown>) }));
    const last = blocks.at(-1);
    if (last) last.cache_control = { type: 'ephemeral' };
    return { ...m, content: blocks as unknown as Anthropic.MessageParam['content'] };
  });

export const runTurn = async (
  clients: LlmClients,
  prompt: PhasePrompt,
  tools: Anthropic.Messages.Tool[],
  system: string,
  messages: Anthropic.MessageParam[],
  usage: Usage,
): Promise<TurnOutput> => {
  // 어느 경로로 나갈지는 모델이 정한다. 대화 상태는 양쪽 다 Anthropic 블록 형태 하나다.
  if (!isAnthropicModel(prompt.model)) return runTurnCompat(clients.compat, prompt, tools, system, messages, usage);

  const systemBlocks: Anthropic.TextBlockParam[] = [{ type: 'text', text: prompt.system, cache_control: { type: 'ephemeral' } }];
  if (system) systemBlocks.push({ type: 'text', text: system, cache_control: { type: 'ephemeral' } });

  const params = {
    model: modelId(prompt.model),
    max_tokens: MAX_OUTPUT_TOKENS,
    tools,
    tool_choice: { type: 'auto' as const },
    system: systemBlocks,
    messages: withLastBlockCached(messages),
    // 5계열 기본 display가 omitted라 thinking 본문이 빈 채로 온다. 요약을 받아 원장 턴
    // 기록에 싣는다 — 비용은 동일하고, 블록은 어차피 서명째 왕복하므로 대화 계약도 그대로다.
    thinking: { type: 'adaptive', display: 'summarized' } as never,
    ...(prompt.effort && { output_config: { effort: prompt.effort as never } }),
  };

  const message = await clients.anthropic.messages.stream(params, SKIP_CACHE).finalMessage();

  usage.calls += 1;
  const fresh = message.usage.input_tokens ?? 0;
  const write = message.usage.cache_creation_input_tokens ?? 0;
  const read = message.usage.cache_read_input_tokens ?? 0;
  usage.promptTokens += fresh + write + read;
  usage.cacheWriteTokens += write;
  usage.cachedTokens += read;
  usage.completionTokens += message.usage.output_tokens ?? 0;

  const toolUses = message.content
    .filter((b): b is Anthropic.ToolUseBlock => b.type === 'tool_use')
    .map((b) => ({ id: b.id, name: b.name, input: b.input }));

  return { content: message.content as unknown[], toolUses };
};

export type SearchExecutor = (query: string) => Promise<{ content: string; hits: number }>;

export type ExecutedTools = {
  results: { toolUseId: string; content: string }[];
  records: ToolRecord[];
};

export const executeSearches = async (toolUses: ToolUse[], turn: number, search: SearchExecutor | null): Promise<ExecutedTools> => {
  const results: { toolUseId: string; content: string }[] = [];
  const records: ToolRecord[] = [];

  for (const use of toolUses) {
    if (use.name === 'search') {
      const input = use.input as { query: string };
      if (!search) {
        results.push({ toolUseId: use.id, content: '이 단계에서는 검색을 쓸 수 없습니다.' });
        continue;
      }
      // 검색 실패는 삼키지 않되 도구 결과로 돌려준다 — 재시도·우회는 에이전트 판단이다.
      try {
        const r = await search(input.query);
        records.push({ turn, tool: 'search', query: input.query, hits: r.hits });
        results.push({ toolUseId: use.id, content: r.content });
      } catch (err) {
        records.push({ turn, tool: 'search', query: input.query, hits: 0 });
        results.push({ toolUseId: use.id, content: `검색 실패: ${String(err).slice(0, 200)}` });
      }
      continue;
    }
    results.push({ toolUseId: use.id, content: `알 수 없는 도구: ${use.name}` });
  }

  return { results, records };
};
