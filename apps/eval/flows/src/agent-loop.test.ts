import { describe, expect, it } from 'vitest';
import { executeToolUses, runTurn } from './agent-loop.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { AnalysisStagePrompt } from '../../src/lib/domain/analysis-prompts.ts';
import type { Usage } from './analysis-llm.ts';

const PROMPT: AnalysisStagePrompt = { system: '시스템', model: 'anthropic/claude-opus-5', effort: 'medium', temperature: null };
const CONTENT = '홍길동은 문을 열었다. 안내 방송. 다시 안내 방송이었다.';

const fakeClient = (message: unknown): Anthropic =>
  ({ messages: { stream: () => ({ finalMessage: async () => message }) } }) as unknown as Anthropic;

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

describe('executeToolUses', () => {
  it('read·grep을 실행해 결과와 원장 기록을 만든다', async () => {
    const out = await executeToolUses(
      CONTENT,
      [
        { id: 'a', name: 'read', input: { start: 0, end: 10 } },
        { id: 'b', name: 'grep', input: { pattern: '안내 방송' } },
      ],
      3,
      null,
    );
    expect(out.records).toEqual([
      { turn: 3, tool: 'read', start: 0, end: 10 },
      { turn: 3, tool: 'grep', pattern: '안내 방송', total: 2 },
    ]);
    expect(out.results[0].content).toContain('[0~10]');
    expect(out.results[1].content).toContain('총 2건');
    expect(out.submissions).toEqual([]);
  });

  it('정규식 오류와 무매치를 안내 문구로 돌려준다', async () => {
    const out = await executeToolUses(
      CONTENT,
      [
        { id: 'a', name: 'grep', input: { pattern: '[깨진' } },
        { id: 'b', name: 'grep', input: { pattern: '없는패턴xyz' } },
      ],
      0,
      null,
    );
    expect(out.results[0].content).toContain('정규식 오류');
    expect(out.results[1].content).toContain('부재의 증거가 아니다');
  });

  it('제출 도구는 실행하지 않고 submissions로 돌려준다', async () => {
    const out = await executeToolUses(CONTENT, [{ id: 's', name: 'file_finding', input: { axis: 'a' } }], 1, null);
    expect(out.submissions).toHaveLength(1);
    expect(out.results).toEqual([]);
  });

  it('검색 콜백 결과와 실패를 도구 결과로 만든다', async () => {
    const ok = await executeToolUses(CONTENT, [{ id: 'q', name: 'search', input: { query: '홍길동전' } }], 2, async () => ({
      content: '검색 결과입니다',
      hits: 3,
    }));
    expect(ok.records).toEqual([{ turn: 2, tool: 'search', query: '홍길동전', hits: 3 }]);
    expect(ok.results[0].content).toBe('검색 결과입니다');

    const fail = await executeToolUses(CONTENT, [{ id: 'q', name: 'search', input: { query: 'x' } }], 2, async () => {
      throw new Error('막힘');
    });
    expect(fail.results[0].content).toContain('검색 실패');
  });
});
