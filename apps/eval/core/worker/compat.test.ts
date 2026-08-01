import { describe, expect, it } from 'vitest';
import { addCompatUsage, callToolCompat, runTurnCompat, toCompatMessages, toCompatTools, turnFromCompat } from './compat.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type OpenAI from 'openai';
import type { PhasePrompt, Usage } from '../contracts.ts';

const PROMPT: PhasePrompt = { system: '시스템', model: 'google-vertex-ai/google/gemini-3.6-flash', effort: 'low' };

const usage = (): Usage => ({ calls: 0, promptTokens: 0, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0 });

const READ: Anthropic.Messages.Tool = {
  name: 'read',
  description: '구간 열람',
  input_schema: {
    type: 'object',
    properties: { start: { type: 'number' }, end: { type: 'number' } },
    required: ['start', 'end'],
    additionalProperties: false,
  },
};

const fakeCompat = (...responses: unknown[]): { client: OpenAI; captured: unknown[] } => {
  const captured: unknown[] = [];
  const client = {
    chat: {
      completions: {
        create: async (params: unknown) => {
          captured.push(params);
          return responses[Math.min(captured.length - 1, responses.length - 1)];
        },
      },
    },
  } as unknown as OpenAI;
  return { client, captured };
};

describe('toCompatMessages', () => {
  it('블록 대화를 chat 형태로 바꾼다 — tool_result는 role:tool로, cache_control은 떨어진다', () => {
    const out = toCompatMessages(
      ['시스템', null, '규약'],
      [
        { role: 'user', content: '시작' },
        {
          role: 'assistant',
          content: [
            { type: 'text', text: '읽겠습니다' },
            { type: 'tool_use', id: 't1', name: 'read', input: { start: 0, end: 10 } },
          ] as never,
        },
        {
          role: 'user',
          content: [{ type: 'tool_result', tool_use_id: 't1', content: '[0~10]\n본문', cache_control: { type: 'ephemeral' } }] as never,
        },
        { role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' },
      ],
    );
    expect(out).toEqual([
      { role: 'system', content: '시스템\n\n규약' },
      { role: 'user', content: '시작' },
      {
        role: 'assistant',
        content: '읽겠습니다',
        tool_calls: [{ id: 't1', type: 'function', function: { name: 'read', arguments: '{"start":0,"end":10}' } }],
      },
      { role: 'tool', tool_call_id: 't1', content: '[0~10]\n본문' },
      { role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' },
    ]);
  });

  it('본문 없는 어시스턴트 턴은 content null로 낸다', () => {
    const out = toCompatMessages(
      ['s'],
      [{ role: 'assistant', content: [{ type: 'tool_use', id: 't1', name: 'read', input: {} }] as never }],
    );
    expect(out[1]).toEqual({
      role: 'assistant',
      content: null,
      tool_calls: [{ id: 't1', type: 'function', function: { name: 'read', arguments: '{}' } }],
    });
  });
});

describe('toCompatTools', () => {
  it('function 도구로 바꾸고 strict는 옮기지 않는다', () => {
    const out = toCompatTools([{ ...READ, strict: true } as never]);
    expect(out).toEqual([{ type: 'function', function: { name: 'read', description: '구간 열람', parameters: READ.input_schema } }]);
  });
});

describe('turnFromCompat', () => {
  it('tool_calls를 tool_use 블록으로 되돌리고, 인자 파싱 실패는 빈 입력으로 둔다', () => {
    const out = turnFromCompat({
      content: '읽겠습니다',
      tool_calls: [
        { id: 'a', type: 'function', function: { name: 'read', arguments: '{"start":1,"end":2}' } },
        { id: 'b', type: 'function', function: { name: 'grep', arguments: '{깨진 json' } },
      ],
    } as never);
    expect(out.toolUses).toEqual([
      { id: 'a', name: 'read', input: { start: 1, end: 2 } },
      { id: 'b', name: 'grep', input: {} },
    ]);
    expect(out.content[0]).toEqual({ type: 'text', text: '읽겠습니다' });
  });

  it('gemini thought_signature를 블록에 실었다가 되돌릴 때 복원한다', () => {
    const sig = { google: { thought_signature: 'SIG' } };
    const out = turnFromCompat({
      content: null,
      tool_calls: [{ id: 'a', type: 'function', function: { name: 'read', arguments: '{"start":0,"end":4}' }, extra_content: sig }],
    } as never);
    expect(out.content[0]).toMatchObject({ type: 'tool_use', id: 'a', extra_content: sig });

    const msgs = toCompatMessages(['s'], [{ role: 'assistant', content: out.content as never }]);
    expect(msgs[1]).toMatchObject({
      role: 'assistant',
      tool_calls: [{ id: 'a', type: 'function', extra_content: sig }],
    });
  });

  it('빈 어시스턴트 턴은 이력에서 뺀다 — Gemini가 빈 parts를 거부한다', () => {
    const msgs = toCompatMessages(
      ['s'],
      [
        { role: 'user', content: '시작' },
        { role: 'assistant', content: [] as never },
        { role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' },
      ],
    );
    expect(msgs).toEqual([
      { role: 'system', content: 's' },
      { role: 'user', content: '시작' },
      { role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' },
    ]);
  });

  it('본문에 실린 message 수준 서명도 왕복한다', () => {
    const sig = { google: { thought_signature: 'MSG-SIG' } };
    const out = turnFromCompat({ content: '생각했습니다', extra_content: sig } as never);
    expect(out.content[0]).toMatchObject({ type: 'text', extra_content: sig });
    const msgs = toCompatMessages(['s'], [{ role: 'assistant', content: out.content as never }]);
    expect(msgs[1]).toMatchObject({ role: 'assistant', content: '생각했습니다', extra_content: sig });
  });
});

describe('addCompatUsage', () => {
  it('매핑 D — prompt는 입력 총량, 캐시 쓰기는 더하지 않는다', () => {
    const u = usage();
    addCompatUsage(u, { prompt_tokens: 100, completion_tokens: 7, prompt_tokens_details: { cached_tokens: 60 } } as never);
    expect(u).toEqual({ calls: 1, promptTokens: 100, completionTokens: 7, cachedTokens: 60, cacheWriteTokens: 0 });
  });
});

describe('runTurnCompat', () => {
  it('모델 문자열을 그대로 보내고 effort를 reasoning_effort로 싣는다', async () => {
    const { client, captured } = fakeCompat({
      choices: [
        {
          message: {
            content: null,
            tool_calls: [{ id: 'a', type: 'function', function: { name: 'read', arguments: '{"start":0,"end":4}' } }],
          },
        },
      ],
      usage: { prompt_tokens: 10, completion_tokens: 2 },
    });
    const u = usage();
    const out = await runTurnCompat(client, PROMPT, [READ], '규약', [{ role: 'user', content: '시작' }], u);
    const params = captured[0] as Record<string, unknown>;
    expect(params.model).toBe('google-vertex-ai/google/gemini-3.6-flash');
    expect(params.reasoning_effort).toBe('low');
    expect(params.tool_choice).toBe('auto');
    expect(out.toolUses).toEqual([{ id: 'a', name: 'read', input: { start: 0, end: 4 } }]);
    expect(u.calls).toBe(1);
  });
});

describe('runTurnCompat — 빈 응답', () => {
  it('빈 응답은 즉시 1회 재시도하고, 두 번째가 정상이면 그것을 쓴다', async () => {
    const { client, captured } = fakeCompat(
      { choices: [{ message: { content: null } }], usage: { prompt_tokens: 1, completion_tokens: 0 } },
      {
        choices: [{ message: { content: null, tool_calls: [{ id: 'a', type: 'function', function: { name: 'read', arguments: '{}' } }] } }],
        usage: { prompt_tokens: 1, completion_tokens: 1 },
      },
    );
    const u = usage();
    const out = await runTurnCompat(client, PROMPT, [READ], '', [{ role: 'user', content: '시작' }], u);
    expect(captured).toHaveLength(2);
    expect(out.toolUses).toHaveLength(1);
    expect(u.calls).toBe(2);
  });

  it('두 번 다 비면 빈 턴을 그대로 돌려준다 — 루프의 재촉이 잇는다', async () => {
    const { client, captured } = fakeCompat({
      choices: [{ message: { content: null } }],
      usage: { prompt_tokens: 1, completion_tokens: 0 },
    });
    const out = await runTurnCompat(client, PROMPT, [READ], '', [{ role: 'user', content: '시작' }], usage());
    expect(captured).toHaveLength(2);
    expect(out).toEqual({ content: [], toolUses: [] });
  });
});

describe('callToolCompat', () => {
  it('강제 호출로 받고, 스키마 위반은 한 번 지적해 다시 받는다', async () => {
    const { client, captured } = fakeCompat(
      {
        choices: [
          { message: { content: null, tool_calls: [{ id: 'a', type: 'function', function: { name: 'read', arguments: '{"start":0}' } }] } },
        ],
        usage: { prompt_tokens: 5, completion_tokens: 1 },
      },
      {
        choices: [
          {
            message: {
              content: null,
              tool_calls: [{ id: 'b', type: 'function', function: { name: 'read', arguments: '{"start":0,"end":4}' } }],
            },
          },
        ],
        usage: { prompt_tokens: 6, completion_tokens: 1 },
      },
    );
    const u = usage();
    const out = await callToolCompat<{ start: number; end: number }>(client, PROMPT, READ, ['시스템'], '제출하세요', u);
    expect(out).toEqual({ start: 0, end: 4 });
    expect(captured).toHaveLength(2);
    const first = captured[0] as Record<string, unknown>;
    expect(first.tool_choice).toEqual({ type: 'function', function: { name: 'read' } });
    const second = captured[1] as { messages: unknown[] };
    expect(second.messages.at(-1)).toMatchObject({ role: 'tool', tool_call_id: 'a' });
    expect(u.calls).toBe(2);
  });

  it('도구 호출이 없으면 실패를 표면화한다', async () => {
    const { client } = fakeCompat({
      choices: [{ message: { content: '못 하겠습니다' } }],
      usage: { prompt_tokens: 1, completion_tokens: 1 },
    });
    await expect(callToolCompat(client, PROMPT, READ, ['시스템'], '제출하세요', usage())).rejects.toThrow('도구 호출 없음');
  });
});
