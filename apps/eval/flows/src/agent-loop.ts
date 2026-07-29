// 에이전틱 턴 실행기 — 스펙 §4. 모델 턴 하나를 돌리고, 로컬 도구(read/grep)와
// 검색을 실행해 tool_result로 되돌린다. 제출 도구는 실행하지 않고 워크플로에 넘긴다 —
// 검증·반려는 단계의 계약이지 루프의 일이 아니다.
import { executeGrep, executeRead } from './manuscript-tools.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { AnalysisStagePrompt } from '../../src/lib/domain/analysis-prompts.ts';
import type { Usage } from './analysis-llm.ts';
import type { ToolRecord } from './editorial-ledger.ts';

const MAX_OUTPUT_TOKENS = 64_000;
const SKIP_CACHE = { headers: { 'cf-aig-skip-cache': 'true' } };

const modelId = (model: string): string => model.replace(/^anthropic\//, '');

export type ToolUse = { id: string; name: string; input: unknown };
export type TurnOutput = { content: unknown[]; toolUses: ToolUse[] };

const SUBMISSION_TOOLS = new Set(['submit_research', 'submit_plan', 'file_finding', 'file_strength', 'submit_review']);

// 대화 접두부가 턴마다 캐시되도록 마지막 메시지의 마지막 블록에 표시를 옮겨 단다.
// 이것이 없으면 대화 비용이 턴 수에 제곱으로 는다.
const withLastBlockCached = (messages: Anthropic.MessageParam[]): Anthropic.MessageParam[] =>
  messages.map((m, i) => {
    if (i !== messages.length - 1) return m;
    const blocks: Record<string, unknown>[] =
      typeof m.content === 'string' ? [{ type: 'text', text: m.content }] : m.content.map((b) => ({ ...(b as Record<string, unknown>) }));
    const last = blocks.at(-1);
    if (last) last.cache_control = { type: 'ephemeral' };
    return { ...m, content: blocks as unknown as Anthropic.MessageParam['content'] };
  });

export const runTurn = async (
  client: Anthropic,
  prompt: AnalysisStagePrompt,
  tools: Anthropic.Messages.Tool[],
  system: string,
  messages: Anthropic.MessageParam[],
  usage: Usage,
): Promise<TurnOutput> => {
  const systemBlocks: Anthropic.TextBlockParam[] = [{ type: 'text', text: prompt.system, cache_control: { type: 'ephemeral' } }];
  if (system) systemBlocks.push({ type: 'text', text: system, cache_control: { type: 'ephemeral' } });

  const params = {
    model: modelId(prompt.model),
    max_tokens: MAX_OUTPUT_TOKENS,
    tools,
    tool_choice: { type: 'auto' as const },
    system: systemBlocks,
    messages: withLastBlockCached(messages),
    ...(prompt.effort && { output_config: { effort: prompt.effort as never } }),
  };

  const message = await client.messages.stream(params, SKIP_CACHE).finalMessage();

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
  submissions: ToolUse[];
  records: ToolRecord[];
};

const renderReadResult = (r: ReturnType<typeof executeRead>): string =>
  `[${r.start}~${r.end}]${r.truncated ? ' (상한으로 잘림 — 이어서 read 하세요)' : ''}\n${r.text}`;

const renderGrepResult = (r: ReturnType<typeof executeGrep>): string => {
  if (r.error) return r.error;
  if (r.total === 0) return '매치 없음 — 무매치는 부재의 증거가 아니다. 변형 패턴을 더 시도하거나 구간을 열람해 확인하라.';
  const head = `총 ${r.total}건${r.total > r.matches.length ? ` (앞 ${r.matches.length}건만 표시)` : ''}`;
  return [head, ...r.matches.map((m) => `[${m.start}~${m.end}] …${m.context}…`)].join('\n');
};

export const executeToolUses = async (
  content: string,
  toolUses: ToolUse[],
  turn: number,
  search: SearchExecutor | null,
): Promise<ExecutedTools> => {
  const results: { toolUseId: string; content: string }[] = [];
  const submissions: ToolUse[] = [];
  const records: ToolRecord[] = [];

  for (const use of toolUses) {
    if (SUBMISSION_TOOLS.has(use.name)) {
      submissions.push(use);
      continue;
    }
    if (use.name === 'read') {
      const input = use.input as { start: number; end: number };
      const r = executeRead(content, input.start, input.end);
      records.push({ turn, tool: 'read', start: r.start, end: r.end });
      results.push({ toolUseId: use.id, content: renderReadResult(r) });
      continue;
    }
    if (use.name === 'grep') {
      const input = use.input as { pattern: string };
      const r = executeGrep(content, input.pattern);
      records.push({ turn, tool: 'grep', pattern: input.pattern, total: r.total });
      results.push({ toolUseId: use.id, content: renderGrepResult(r) });
      continue;
    }
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

  return { results, submissions, records };
};
