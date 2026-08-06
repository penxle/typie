import { describe, expect, it } from 'vitest';
import { executeSearches, runTurn } from './agent-loop.ts';
import type { PhasePrompt, Usage } from '../contracts.ts';
import type { LlmClients } from './compat.ts';

const PROMPT: PhasePrompt = { system: '시스템', model: 'anthropic/claude-opus-5', effort: 'medium' };

const fakeClient = (message: unknown): LlmClients =>
  ({ anthropic: { messages: { stream: () => ({ finalMessage: async () => message }) } }, compat: {} }) as unknown as LlmClients;

const usage = (): Usage => ({ calls: 0, promptTokens: 0, completionTokens: 0, cachedTokens: 0, cacheWriteTokens: 0 });

describe('runTurn', () => {
  it('tool_use 블록을 뽑고 usage를 합산한다', async () => {
    const u = usage();
    const out = await runTurn(
      fakeClient({
        content: [
          { type: 'text', text: '읽겠습니다' },
          { type: 'tool_use', id: 't1', name: 'read', input: { start: 0, end: 10 } },
        ],
        usage: { input_tokens: 10, cache_creation_input_tokens: 5, cache_read_input_tokens: 20, output_tokens: 7 },
      }),
      PROMPT,
      [],
      '규약',
      [{ role: 'user', content: '시작' }],
      u,
    );
    expect(out.toolUses).toEqual([{ id: 't1', name: 'read', input: { start: 0, end: 10 } }]);
    expect(u.calls).toBe(1);
    expect(u.promptTokens).toBe(35);
    expect(u.completionTokens).toBe(7);
  });
});

describe('executeSearches', () => {
  it('검색 콜백 결과와 실패를 도구 결과로 만든다', async () => {
    const ok = await executeSearches([{ id: 'q', name: 'search', input: { query: '홍길동전' } }], 2, async () => ({
      content: '검색 결과입니다',
      hits: 3,
    }));
    expect(ok.records).toEqual([{ turn: 2, tool: 'search', query: '홍길동전', hits: 3 }]);
    expect(ok.results[0].content).toBe('검색 결과입니다');

    const fail = await executeSearches([{ id: 'q', name: 'search', input: { query: 'x' } }], 2, async () => {
      throw new Error('막힘');
    });
    expect(fail.results[0].content).toContain('검색 실패');
  });

  it('검색 실행기가 없으면 안내한다', async () => {
    const out = await executeSearches([{ id: 'q', name: 'search', input: { query: 'x' } }], 0, null);
    expect(out.records).toEqual([]);
    expect(out.results[0].content).toContain('검색을 쓸 수 없습니다');
  });

  it('검색이 아닌 도구는 실행하지 않고 알린다 — 파일·제출 도구는 스테이지 루프의 몫이다', async () => {
    const out = await executeSearches([{ id: 'a', name: 'read', input: { path: 'manuscript/doc-1.txt', start: 0, end: 10 } }], 0, null);
    expect(out.records).toEqual([]);
    expect(out.results[0].content).toContain('알 수 없는 도구');
  });
});
